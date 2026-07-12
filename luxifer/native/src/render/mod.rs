//! Frame-Koordination und GPU-Ressourcen des nativen Renderers. Besitzt die
//! GPU, den egui-Wgpu-Renderer/-State, den Bild-Store und den Vertex-Cache.
//!
//! Die Trennung zu `App`: Der App-Root baut den egui-Frame (dazu braucht die
//! `ui::build`-Closure `&mut App`) und übergibt das Ergebnis samt Szenenzustand
//! an [`Renderer::draw_frame`], das den eigentlichen GPU-Frame erzeugt. So
//! liegen GPU-Ressourcen und Frame-Ablauf gebündelt hier, nicht im Monolithen.

use std::time::Instant;

use egui_wgpu::ScreenDescriptor;
use luxifer_application::EditorSession;
use winit::window::Window;

use crate::camera::Camera;
use crate::canvas::overlay::{overlay_vertices, OverlayInput};
use crate::canvas::scene::{base_vertices, preview_vertices};
use crate::gpu::Gpu;
use crate::image_gpu::ImageStore;

/// Nur-lesender Szenenzustand, den der Root pro Frame an den Renderer übergibt.
pub struct FrameScene<'a> {
    pub session: &'a EditorSession,
    pub cam: &'a Camera,
    pub overlay: OverlayInput<'a>,
    /// Ob externe Ereignisse (Import) neue Bild-Texturen nötig machen.
    pub image_dirty: bool,
    pub preview: bool,
    pub selection_only: bool,
}

pub struct Renderer {
    gpu: Gpu,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    images: ImageStore,
    // Vertex-Cache: die (teure) Scanline-Füllung wird NUR neu gebaut, wenn sich
    // der Zustand ändert — nicht pro Frame. Pan/Zoom lassen die Vertices
    // unberührt (die Projektion macht der Shader), daher bleiben sie gecacht.
    verts: Vec<crate::scene_geo::Vertex>,
    background_end: u32,
    /// Render-Revision (aus dem Core) beim letzten Vertex-Aufbau.
    last_render_rev: u64,
    last_frame: Instant,
    fps: f32,
    /// Ob egui im letzten Frame sofort weiter zeichnen wollte.
    wants_repaint: bool,
}

impl Renderer {
    pub fn new(gpu: Gpu, egui_state: egui_winit::State) -> Self {
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);
        Self {
            gpu,
            egui_state,
            egui_renderer,
            images: ImageStore::default(),
            verts: Vec::new(),
            background_end: 0,
            // MAX erzwingt den Aufbau im ersten Frame (Core startet bei 0).
            last_render_rev: u64::MAX,
            last_frame: Instant::now(),
            fps: 0.0,
            wants_repaint: false,
        }
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn wants_repaint(&self) -> bool {
        self.wants_repaint
    }

    /// Erzwingt den Vertex-Neuaufbau im nächsten Frame (z. B. nach Projektwechsel,
    /// weil der geladene Zustand einen eigenen Revisionszähler mitbringt).
    pub fn invalidate_scene(&mut self) {
        self.last_render_rev = u64::MAX;
    }

    /// Nimmt egui-Roheingaben entgegen (für den Frame-Aufbau im Root).
    pub fn take_egui_input(&mut self, window: &Window) -> egui::RawInput {
        self.egui_state.take_egui_input(window)
    }

    /// Leitet ein Fensterereignis an egui weiter (Fokus, Hover, Tastatur).
    pub fn on_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.egui_state.on_window_event(window, event)
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.gpu.resize(w, h);
    }

    /// Zeichnet einen Frame: aktualisiert Vertex-/Bild-Caches aus `scene`, lädt
    /// egui-Daten hoch und rendert Canvas + Overlay + egui. `full`/`tris` liefert
    /// der Root (er besitzt den egui-`Context` und baut damit den Frame).
    pub fn draw_frame(
        &mut self,
        window: &Window,
        scene: FrameScene,
        full: egui::FullOutput,
        tris: Vec<egui::ClippedPrimitive>,
    ) {
        // FPS.
        let dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();
        if dt > 0.0 {
            self.fps = 0.9 * self.fps + 0.1 * (1.0 / dt);
        }

        self.egui_state
            .handle_platform_output(window, full.platform_output);
        self.wants_repaint = full
            .viewport_output
            .values()
            .any(|v| v.repaint_delay.is_zero());

        // Canvas-Vertices nur neu bauen+hochladen, wenn sich die Szene änderte.
        let rev = scene.session.render_rev();
        let scene_changed = rev != self.last_render_rev;
        if scene_changed {
            self.last_render_rev = rev;
            let geometry = if scene.preview {
                preview_vertices(scene.session, scene.selection_only)
            } else {
                base_vertices(scene.session)
            };
            self.background_end = geometry.background_end;
            self.verts = geometry.vertices;
            let verts = std::mem::take(&mut self.verts);
            self.gpu.upload_verts(&verts);
            self.verts = verts;
        }
        self.gpu.upload_camera(scene.cam);
        if scene.image_dirty || scene_changed {
            self.images.sync(
                &self.gpu.device,
                &self.gpu.queue,
                self.gpu.config.format,
                scene.session,
            );
        }
        let count = self.verts.len() as u32;
        let overlay = if scene.preview {
            Vec::new()
        } else {
            overlay_vertices(&scene.overlay)
        };
        self.gpu.upload_overlay(&overlay);

        let frame = match self.gpu.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());

        // egui-Texturen/Buffer aktualisieren.
        let screen = ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point: full.pixels_per_point,
        };
        for (id, delta) in &full.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
        }
        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut enc,
            &tris,
            &screen,
        );

        // Scratch-Buffer für die Bild-Quads (muss den Render-Pass überleben).
        let mut img_scratch: Option<wgpu::Buffer> = None;
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.06,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Opakes Bett/Gitter zuerst. Danach echte Bildtexturen, anschließend
            // Vektor-Fills und Konturen; Handles bleiben ganz oben.
            self.gpu.draw_canvas_range(&mut rp, 0..self.background_end);
            self.images.draw(
                &mut rp,
                &self.gpu,
                scene.cam,
                scene.session,
                scene.preview && scene.selection_only,
                &mut img_scratch,
            );
            self.gpu
                .draw_canvas_range(&mut rp, self.background_end..count);
            self.gpu.draw_overlay(&mut rp);
            // egui obendrauf (eigener Lebenszeit-Scope via forget_lifetime).
            let mut rp = rp.forget_lifetime();
            self.egui_renderer.render(&mut rp, &tris, &screen);
        }
        self.gpu.queue.submit(Some(enc.finish()));
        frame.present();

        for id in &full.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }
}
