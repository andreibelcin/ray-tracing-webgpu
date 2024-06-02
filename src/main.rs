use std::sync::Arc;

use camera::Camera;
use util::{Point3, Vec3};
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BlendState, Buffer, BufferBinding, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoderDescriptor, ComputePassDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, DeviceDescriptor, Extent3d, Instance, InstanceDescriptor,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, Sampler, SamplerDescriptor, ShaderModule,
    ShaderStages, Surface, SurfaceConfiguration, SurfaceError, Texture, TextureDescriptor,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

mod camera;
mod util;

struct WebGPUResources<'a> {
    surface: Surface<'a>,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
}

impl<'a> WebGPUResources<'a> {
    fn new(window: Arc<Window>) -> Self {
        let instance = Instance::new(InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();

        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default(), None)).unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats[0];

        let size = window.inner_size();

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        Self {
            surface,
            surface_config,
            device,
            queue,
        }
    }

    fn resize_surface(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

struct Scene {
    camera: Camera,
}

struct App<'a> {
    window: Arc<Window>,
    size: PhysicalSize<u32>,
    webgpu_resources: WebGPUResources<'a>,

    compute_pipeline: ComputePipeline,
    compute_bind_group: BindGroup,
    compute_bind_group_layout: BindGroupLayout,

    render_pipeline: RenderPipeline,
    render_bind_group: BindGroup,
    render_bind_group_layout: BindGroupLayout,

    resolution_uniform: Buffer,
    sampler: Sampler,

    scene: Scene,
}

impl<'a> App<'a> {
    fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        let webgpu_resources = WebGPUResources::new(window.clone());

        let resolution_uniform =
            webgpu_resources
                .device
                .create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&[size.width as f32, size.height as f32]),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });

        let compute_texture = webgpu_resources.device.create_texture(&TextureDescriptor {
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        let compute_bind_group_layout =
            webgpu_resources
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                view_dimension: wgpu::TextureViewDimension::D2,
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: TextureFormat::Rgba8Unorm,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let compute_bind_group = webgpu_resources
            .device
            .create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &compute_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &compute_texture.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(
                            resolution_uniform.as_entire_buffer_binding(),
                        ),
                    },
                ],
            });

        let compute_shader = webgpu_resources
            .device
            .create_shader_module(include_wgsl!("compute.wgsl"));
        let compute_pipeline_layout =
            webgpu_resources
                .device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let compute_pipeline =
            webgpu_resources
                .device
                .create_compute_pipeline(&ComputePipelineDescriptor {
                    module: &compute_shader,
                    entry_point: "main",
                    compilation_options: PipelineCompilationOptions::default(),
                    label: None,
                    layout: Some(&compute_pipeline_layout),
                });

        let sampler = webgpu_resources
            .device
            .create_sampler(&SamplerDescriptor::default());
        let render_bind_group_layout =
            webgpu_resources
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let render_bind_group = webgpu_resources
            .device
            .create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &render_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &compute_texture.create_view(&TextureViewDescriptor::default()),
                        ),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let render_shader = webgpu_resources
            .device
            .create_shader_module(include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            webgpu_resources
                .device
                .create_pipeline_layout(&PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&render_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let render_pipeline =
            webgpu_resources
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: "vert_main",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader,
                        entry_point: "frag_main",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: webgpu_resources.surface_config.format,
                            blend: Some(BlendState::REPLACE),
                            write_mask: ColorWrites::all(),
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                });

        Self {
            window,
            size,
            webgpu_resources,
            compute_bind_group,
            compute_bind_group_layout,
            compute_pipeline,
            render_pipeline,
            render_bind_group,
            render_bind_group_layout,
            resolution_uniform,
            sampler,
            scene: Scene {
                camera: Camera {
                    origin: Point3::origin(),
                    direction: -Vec3::k(),
                },
            },
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.webgpu_resources.resize_surface(new_size);

        let compute_texture = self
            .webgpu_resources
            .device
            .create_texture(&TextureDescriptor {
                size: Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                label: None,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[TextureFormat::Rgba8Unorm],
            });

        self.webgpu_resources.queue.write_buffer(
            &self.resolution_uniform,
            0,
            bytemuck::cast_slice(&[self.size.width as f32, self.size.height as f32]),
        );

        self.compute_bind_group =
            self.webgpu_resources
                .device
                .create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &self.compute_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &compute_texture.create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Buffer(
                                self.resolution_uniform.as_entire_buffer_binding(),
                            ),
                        },
                    ],
                });
        self.render_bind_group =
            self.webgpu_resources
                .device
                .create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout: &self.render_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &compute_texture.create_view(&TextureViewDescriptor::default()),
                            ),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                })
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.webgpu_resources.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .webgpu_resources
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.size.width, self.size.height, 1);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.webgpu_resources.queue.submit([encoder.finish()]);
        output.present();

        Ok(())
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                self.update();
                match self.render() {
                    Ok(_) => {}
                    Err(SurfaceError::Lost) => self.resize(self.size),
                    Err(SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{:?}", e),
                }
                self.window.request_redraw();
            }
            WindowEvent::Resized(new_size) => self.resize(new_size),
            _ => (),
        }
    }
}

#[derive(Default)]
struct AppHolder<'a> {
    title: &'static str,
    app: Option<App<'a>>,
}

impl<'a> ApplicationHandler for AppHolder<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title(self.title))
            .unwrap();
        window.request_redraw();

        self.app = Some(App::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else { return };

        if window_id != app.window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.app = None;
                event_loop.exit();
            }

            event => app.window_event(event_loop, event),
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = AppHolder::default();
    event_loop.run_app(&mut app).unwrap();
}
