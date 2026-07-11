//! Der Anwendungs-Zustand des nativen Editors: hält den Core-`AppState`, die
//! Kamera, das aktive Werkzeug und den GPU/egui-Kontext. Verbindet Eingaben mit
//! Core-Aufrufen (der Core bleibt die Wahrheit) und rendert Canvas + Panels.

use std::sync::Arc;

use egui_wgpu::ScreenDescriptor;
use luxifer_core::geometry::Geo;
use luxifer_core::state::AppState;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

use crate::camera::Camera;
use crate::gpu::Gpu;
use crate::scene_geo::{self, Vertex};
use crate::tools::{Drag, Tool};
use crate::ui;

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub state: AppState,
    pub cam: Camera,
    pub tool: Tool,
    pub drag: Drag,
    /// Aktive Zeichenfarbe für die Palette-Markierung (aus dem Core gespiegelt).
    pub accent: [u8; 3],
    cursor: [f32; 2],
    space_down: bool,
    // Polygon-Zug (Welt-Punkte), bis Doppelklick/Enter schließt.
    poly_pts: Vec<(f64, f64)>,
    // egui.
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    // Panel-Breiten, damit der Canvas den freien Bereich kennt.
    pub left_w: f32,
    pub right_w: f32,
    last_frame: std::time::Instant,
    pub fps: f32,
}

impl App {
    pub fn new(window: Arc<Window>, gpu: Gpu) -> Self {
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);

        let mut state = AppState::new();
        // Ein paar Start-Shapes, damit sofort etwas zu sehen ist.
        state.add_shape(Geo::Rect {
            x: 40.0,
            y: 40.0,
            w: 120.0,
            h: 80.0,
        });
        state.selected.clear();
        state.add_shape(Geo::Ellipse {
            cx: 260.0,
            cy: 120.0,
            rx: 60.0,
            ry: 40.0,
        });
        state.selected.clear();
        let accent = state.active_color().unwrap_or([0x3B, 0x82, 0xF6]);

        let mut cam = Camera::new();
        cam.viewport = [gpu.config.width as f32, gpu.config.height as f32];
        cam.fit_bbox([0.0, 0.0, state.bed_w_mm, state.bed_h_mm], 0.85);

        Self {
            window,
            gpu,
            state,
            cam,
            tool: Tool::Select,
            drag: Drag::None,
            accent,
            cursor: [0.0, 0.0],
            space_down: false,
            poly_pts: Vec::new(),
            egui_ctx,
            egui_state,
            egui_renderer,
            left_w: 0.0,
            right_w: 0.0,
            last_frame: std::time::Instant::now(),
            fps: 0.0,
        }
    }

    pub fn window_event(&mut self, event: &WindowEvent) -> bool {
        // egui zuerst — verschluckt es das Event (Panel getroffen), geht es nicht
        // an den Canvas.
        let resp = self.egui_state.on_window_event(&self.window, event);
        if resp.consumed {
            // Trotzdem Cursor mitschreiben, damit Canvas-Koordinaten stimmen.
            if let WindowEvent::CursorMoved { position, .. } = event {
                self.cursor = [position.x as f32, position.y as f32];
            }
            return resp.repaint;
        }

        match event {
            WindowEvent::Resized(sz) => {
                self.gpu.resize(sz.width, sz.height);
                self.cam.viewport = [sz.width as f32, sz.height as f32];
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = event.physical_key {
                    match code {
                        KeyCode::Space => self.space_down = pressed,
                        KeyCode::Delete | KeyCode::Backspace if pressed => {
                            if !self.state.selected.is_empty() {
                                self.state.delete_selected();
                            }
                        }
                        KeyCode::Escape if pressed => {
                            self.poly_pts.clear();
                            self.state.selected.clear();
                        }
                        KeyCode::Enter if pressed => self.finish_polygon(),
                        KeyCode::KeyV if pressed => self.tool = Tool::Select,
                        KeyCode::KeyR if pressed => self.tool = Tool::Rect,
                        KeyCode::KeyE if pressed => self.tool = Tool::Ellipse,
                        KeyCode::KeyP if pressed => self.tool = Tool::Polygon,
                        KeyCode::KeyZ if pressed => {
                            self.state.undo();
                        }
                        KeyCode::KeyY if pressed => {
                            self.state.redo();
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new = [position.x as f32, position.y as f32];
                self.on_cursor_move(new);
                self.cursor = new;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse(*button, *state == ElementState::Pressed);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let s = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.cam.zoom_at(1.12_f32.powf(s), self.cursor);
            }
            _ => {}
        }
        true
    }

    fn world(&self) -> [f64; 2] {
        self.cam.screen_to_world(self.cursor)
    }

    fn on_mouse(&mut self, button: MouseButton, pressed: bool) {
        let w = self.world();
        match button {
            MouseButton::Middle => {
                self.drag = if pressed { Drag::Pan } else { Drag::None };
            }
            MouseButton::Left if pressed => {
                if self.space_down {
                    self.drag = Drag::Pan;
                    return;
                }
                match self.tool {
                    Tool::Select => self.begin_select(w),
                    Tool::Rect | Tool::Ellipse => self.drag = Drag::DrawBox { start: w },
                    Tool::Polygon => self.poly_pts.push((w[0], w[1])),
                }
            }
            MouseButton::Left => {
                // Loslassen: Zug abschließen.
                self.finish_drag(w);
            }
            _ => {}
        }
    }

    fn begin_select(&mut self, w: [f64; 2]) {
        let tol = 4.0 / self.cam.scale as f64;
        if let Some(idx) = self.state.hit_test(w[0], w[1], tol) {
            if !self.state.selected.contains(&idx) {
                self.state.selected = vec![idx];
            }
            self.drag = Drag::MoveShapes { last: w };
        } else {
            self.state.selected.clear();
            self.drag = Drag::Marquee { start: w };
        }
    }

    fn on_cursor_move(&mut self, new: [f32; 2]) {
        let dx = new[0] - self.cursor[0];
        let dy = new[1] - self.cursor[1];
        match &mut self.drag {
            Drag::Pan => self.cam.pan_pixels(dx, dy),
            Drag::MoveShapes { last } => {
                let w = self.cam.screen_to_world(new);
                self.state
                    .translate_selected(w[0] - last[0], w[1] - last[1]);
                *last = w;
            }
            // Marquee/DrawBox: nur der Endpunkt zählt (Vorschau folgt später).
            _ => {}
        }
    }

    fn finish_drag(&mut self, w: [f64; 2]) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Marquee { start } => {
                if (start[0] - w[0]).abs() > 1.0 || (start[1] - w[1]).abs() > 1.0 {
                    self.state.select_in_rect(start[0], start[1], w[0], w[1]);
                }
            }
            Drag::DrawBox { start } => self.finish_box(start, w),
            Drag::MoveShapes { .. } => {
                self.state.discard_last_undo_if_no_change();
            }
            _ => {}
        }
    }

    fn finish_box(&mut self, a: [f64; 2], b: [f64; 2]) {
        let x = a[0].min(b[0]);
        let y = a[1].min(b[1]);
        let w = (a[0] - b[0]).abs();
        let h = (a[1] - b[1]).abs();
        if w < 0.5 || h < 0.5 {
            return;
        }
        let geo = match self.tool {
            Tool::Ellipse => Geo::Ellipse {
                cx: x + w / 2.0,
                cy: y + h / 2.0,
                rx: w / 2.0,
                ry: h / 2.0,
            },
            _ => Geo::Rect { x, y, w, h },
        };
        let idx = self.state.add_shape(geo);
        self.state.selected = vec![idx];
        self.refresh_accent();
    }

    fn finish_polygon(&mut self) {
        if self.poly_pts.len() >= 3 {
            let pts = std::mem::take(&mut self.poly_pts);
            let idx = self.state.add_shape(Geo::Polyline { pts, closed: true });
            self.state.selected = vec![idx];
            self.refresh_accent();
        } else {
            self.poly_pts.clear();
        }
    }

    pub fn pick_color(&mut self, c: [u8; 3]) {
        self.state.activate_color(c);
        self.refresh_accent();
    }

    fn refresh_accent(&mut self) {
        if let Some(c) = self.state.active_color() {
            self.accent = c;
        }
    }

    /// Baut die Zeichendaten (Tisch, Shapes, Auswahl-BBox, laufendes Polygon).
    fn build_vertices(&self) -> Vec<Vertex> {
        let mut v = scene_geo::rect_outline(
            0.0,
            0.0,
            self.state.bed_w_mm as f32,
            self.state.bed_h_mm as f32,
            scene_geo::BED_COLOR,
        );
        v.extend(scene_geo::shape_lines(&self.state, self.accent));
        if let Some(b) = self.state.selection_bbox() {
            v.extend(scene_geo::rect_outline(
                b.x as f32,
                b.y as f32,
                b.w as f32,
                b.h as f32,
                scene_geo::SEL_BOX_COLOR,
            ));
        }
        // Laufender Polygon-Zug als helle Linie.
        if self.poly_pts.len() >= 2 {
            let col = [0.9, 0.9, 0.95, 1.0];
            for wnd in self.poly_pts.windows(2) {
                v.push(Vertex {
                    pos: [wnd[0].0 as f32, wnd[0].1 as f32],
                    color: col,
                });
                v.push(Vertex {
                    pos: [wnd[1].0 as f32, wnd[1].1 as f32],
                    color: col,
                });
            }
        }
        v
    }

    pub fn render(&mut self) {
        // FPS.
        let dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = std::time::Instant::now();
        if dt > 0.0 {
            self.fps = 0.9 * self.fps + 0.1 * (1.0 / dt);
        }

        // egui-Frame bauen (Panels). Liefert Breiten zurück für den Canvas-Bereich.
        let raw = self.egui_state.take_egui_input(&self.window);
        let full = self.egui_ctx.clone().run(raw, |ctx| ui::build(ctx, self));
        self.egui_state
            .handle_platform_output(&self.window, full.platform_output);
        let tris = self.egui_ctx.tessellate(full.shapes, full.pixels_per_point);

        // Canvas-Vertices.
        let verts = self.build_vertices();
        let count = self.gpu.upload(&verts, &self.cam);

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
            self.gpu.draw_canvas(&mut rp, count);
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
