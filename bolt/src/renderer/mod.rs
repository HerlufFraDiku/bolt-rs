use cgmath::prelude::*;
use wgpu::util::DeviceExt;

pub type Window = winit::window::Window;

// GOALS:
// 1. Draw one quad (DONE)
// 2. Draw rotated quads
// 3. Draw quads batched
// 4. textures babii

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    fn descriptor<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    fn from(matrix: cgmath::Matrix4<f32>) -> Self {
        Self {
            view_projection: matrix.into(),
        }
    }
}

const UNIT_QUAD_VERTS: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, 10.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [10.0, 10.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
    Vertex {
        position: [10.0, 0.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];

const INDICES: &[u16] = &[0, 2, 1, 0, 3, 2];

#[derive(Copy, Clone)]
pub enum Shape2D {
    Quad {
        position: [f32; 3],
        scale: [f32; 3],
        color: [f32; 4],
    },
}

async fn create_gpu_device(
    window: &Window,
) -> (
    wgpu::Device,
    wgpu::Queue,
    wgpu::Surface,
    wgpu::SurfaceConfiguration,
) {
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let target = unsafe { instance.create_surface(window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&target),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: target.get_preferred_format(&adapter).unwrap(),
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Fifo,
    };

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        )
        .await
        .unwrap();

    target.configure(&device, &config);

    (device, queue, target, config)
}

pub struct Renderer2D {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: wgpu::Surface,
    pipeline: wgpu::RenderPipeline,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl Renderer2D {
    pub fn new(window: &Window) -> Self {
        let (device, queue, target, target_config) = pollster::block_on(create_gpu_device(window));
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Renderer2D Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/quad2d.wgsl").into()),
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("camera_bind_group_layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Renderer2D Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Renderer2D Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::descriptor()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: target_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer2D Vertex Buffer"),
            contents: bytemuck::cast_slice(UNIT_QUAD_VERTS),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer2D Index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let size = window.inner_size();
        let view = cgmath::Matrix4::look_at_rh(
            (0.0, 0.0, 2.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
        );
        let half_width = size.width as f32 / 2.0;
        let half_height = size.height as f32 / 2.0;
        let projection = cgmath::ortho(
            -half_width,
            half_width,
            -half_height,
            half_height,
            0.0,
            100.0,
        );

        let camera_uniform = CameraUniform::from(crate::OPENGL_TO_WGPU_MATRIX * projection * view);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer2D Camera uniform buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            device,
            queue,
            target,
            pipeline,
            vertex_buffer,
            index_buffer,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn draw(&self, shape: Shape2D) -> Result<(), wgpu::SurfaceError> {
        let output = self.target.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut verts: Vec<Vertex> = vec![];
        match shape {
            Shape2D::Quad {
                position,
                scale,
                color,
            } => {
                let half_scale_x = scale[0] / 2.0;
                let half_scale_y = scale[1] / 2.0;

                verts.push(Vertex {
                    position: [
                        -half_scale_x + position[0],
                        -half_scale_y + position[1],
                        0.0,
                    ],
                    color,
                });
                verts.push(Vertex {
                    position: [-half_scale_x + position[0], half_scale_y + position[1], 0.0],
                    color,
                });
                verts.push(Vertex {
                    position: [half_scale_x + position[0], half_scale_y + position[1], 0.0],
                    color,
                });
                verts.push(Vertex {
                    position: [half_scale_x + position[0], -half_scale_y + position[1], 0.0],
                    color,
                });
            }
        }

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Renderer2D Vertex Buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
