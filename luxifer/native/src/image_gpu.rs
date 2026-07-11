//! Bild-Rendering im Canvas: importierte Assets (Graustufe) als GPU-Texturen an
//! ihrer mm-Box. Eigene Pipeline (texturiertes Quad), dieselben Kamera-Uniforms
//! wie der Linien-Renderer. Der Core liefert die Pixel (`load_asset_luma`).

use std::collections::HashMap;

use luxifer_core::state::AppState;
use wgpu::util::DeviceExt;

use crate::camera::Camera;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImgVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    center: [f32; 2],
    scale: f32,
    _pad: f32,
    viewport: [f32; 2],
    _pad2: [f32; 2],
}

/// Eine geladene Bild-Textur samt Bind-Group.
struct Tex {
    bind: wgpu::BindGroup,
}

/// Hält Pipeline, Sampler, Uniform-Buffer und die geladenen Texturen je Asset-ID.
#[derive(Default)]
pub struct ImageStore {
    pipeline: Option<wgpu::RenderPipeline>,
    sampler: Option<wgpu::Sampler>,
    tex_layout: Option<wgpu::BindGroupLayout>,
    uniform_buf: Option<wgpu::Buffer>,
    uni_bind: Option<wgpu::BindGroup>,
    textures: HashMap<String, Tex>,
}

impl ImageStore {
    /// Baut die Pipeline lazy (beim ersten Bild). Trennt die einmalige
    /// GPU-Objekt-Erzeugung von der Textur-Verwaltung.
    fn ensure_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self.pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("img_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uni_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uni_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uni_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pl_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uni_layout, &tex_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image"),
            layout: Some(&pl_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImgVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        self.pipeline = Some(pipeline);
        self.sampler = Some(sampler);
        self.tex_layout = Some(tex_layout);
        self.uniform_buf = Some(uniform_buf);
        self.uni_bind = Some(uni_bind);
    }

    /// Lädt fehlende Texturen für alle Image-Shapes aus dem Asset-Store.
    pub fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        state: &AppState,
    ) {
        let dir = luxifer_core::assets_dir();
        for s in &state.shapes {
            if let luxifer_core::Geo::Image { asset, .. } = &s.geo {
                if self.textures.contains_key(asset) {
                    continue;
                }
                self.ensure_pipeline(device, format);
                match luxifer_core::load_asset_luma(&dir, asset) {
                    Ok((luma, w, h)) => {
                        let tex = self.upload_texture(device, queue, &luma, w, h);
                        self.textures.insert(asset.clone(), tex);
                    }
                    Err(e) => log::error!("Asset {asset} laden: {e}"),
                }
            }
        }
    }

    fn upload_texture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        luma: &[u8],
        w: u32,
        h: u32,
    ) -> Tex {
        // Luma → RGBA (grau), damit ein Standard-Format reicht.
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for &g in luma {
            rgba.extend_from_slice(&[g, g, g, 255]);
        }
        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("asset"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &rgba,
        );
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: self.tex_layout.as_ref().unwrap(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.sampler.as_ref().unwrap()),
                },
            ],
        });
        Tex { bind }
    }

    /// Zeichnet alle Image-Shapes als texturierte Quads in den Render-Pass.
    pub fn draw<'a>(
        &'a self,
        rp: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cam: &Camera,
        state: &AppState,
        scratch: &'a mut Option<wgpu::Buffer>,
    ) {
        let (Some(pipeline), Some(uni_buf), Some(uni_bind)) = (
            self.pipeline.as_ref(),
            self.uniform_buf.as_ref(),
            self.uni_bind.as_ref(),
        ) else {
            return;
        };
        // Kamera-Uniforms schreiben.
        let uni = Uniforms {
            center: cam.center,
            scale: cam.scale,
            _pad: 0.0,
            viewport: cam.viewport,
            _pad2: [0.0, 0.0],
        };
        queue.write_buffer(uni_buf, 0, bytemuck::bytes_of(&uni));

        // Alle Image-Quads in einen Vertex-Buffer (6 Vertices je Bild).
        let mut verts: Vec<ImgVertex> = Vec::new();
        let mut ranges: Vec<(String, u32, u32)> = Vec::new();
        for s in &state.shapes {
            if let luxifer_core::Geo::Image {
                asset, x, y, w, h, ..
            } = &s.geo
            {
                if !self.textures.contains_key(asset) {
                    continue;
                }
                let start = verts.len() as u32;
                let (x0, y0, x1, y1) = (*x as f32, *y as f32, (*x + *w) as f32, (*y + *h) as f32);
                // Zwei Dreiecke, UV oben-links = (0,0).
                let quad = [
                    ([x0, y0], [0.0, 0.0]),
                    ([x1, y0], [1.0, 0.0]),
                    ([x1, y1], [1.0, 1.0]),
                    ([x0, y0], [0.0, 0.0]),
                    ([x1, y1], [1.0, 1.0]),
                    ([x0, y1], [0.0, 1.0]),
                ];
                for (pos, uv) in quad {
                    verts.push(ImgVertex { pos, uv });
                }
                ranges.push((asset.clone(), start, start + 6));
            }
        }
        if verts.is_empty() {
            return;
        }
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("img_quads"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        *scratch = Some(buf);
        let buf = scratch.as_ref().unwrap();

        rp.set_pipeline(pipeline);
        rp.set_bind_group(0, uni_bind, &[]);
        rp.set_vertex_buffer(0, buf.slice(..));
        for (asset, start, end) in &ranges {
            if let Some(tex) = self.textures.get(asset) {
                rp.set_bind_group(1, &tex.bind, &[]);
                rp.draw(*start..*end, 0..1);
            }
        }
    }
}

const SHADER: &str = r#"
struct U { center: vec2<f32>, scale: f32, _p: f32, viewport: vec2<f32>, _p2: vec2<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>) -> VOut {
    let px = (p - u.center) * u.scale;
    let ndc = vec2<f32>(px.x / (u.viewport.x * 0.5), -px.y / (u.viewport.y * 0.5));
    var o: VOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs(v: VOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, v.uv);
}
"#;
