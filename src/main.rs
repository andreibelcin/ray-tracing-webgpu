use std::sync::Arc;

use camera::Camera;
use util::{build_compute_pipeline, build_render_pipeline, build_texture, texture_bind_groups};
use wgpu::{
    BindGroup, Color, CommandEncoderDescriptor, ComputePassDescriptor,
    ComputePipeline, Device, DeviceDescriptor, Instance, InstanceDescriptor, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RequestAdapterOptions,
    Sampler, SamplerDescriptor, Surface, SurfaceConfiguration, SurfaceError, TextureUsages,
    TextureViewDescriptor,
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
    render_pipeline: RenderPipeline,

    sampler: Sampler,
    compute_texture_bind_group: BindGroup,
    render_texture_bind_group: BindGroup,
    camera_bind_group: BindGroup,

    scene: Scene,
}

impl<'a> App<'a> {
    fn new(window: Window) -> Self {
        let window = Arc::new(window);
        let size = window.inner_size();

        let webgpu_resources = WebGPUResources::new(window.clone());

        let sampler = webgpu_resources
            .device
            .create_sampler(&SamplerDescriptor::default());
        let compute_texture = build_texture(&webgpu_resources.device, size);

        let [compute_texture_bind_group, render_texture_bind_group] =
            texture_bind_groups(&webgpu_resources.device, &compute_texture, &sampler);

        let camera = Camera::new(size, &webgpu_resources.device);
        let camera_bind_group = camera.bind_group(&webgpu_resources.device);

        let compute_pipeline = build_compute_pipeline(&webgpu_resources.device);
        let render_pipeline = build_render_pipeline(
            &webgpu_resources.device,
            webgpu_resources.surface_config.format,
        );

        Self {
            window,
            size,
            webgpu_resources,
            compute_pipeline,
            render_pipeline,
            sampler,
            compute_texture_bind_group,
            render_texture_bind_group,
            camera_bind_group,
            scene: Scene { camera },
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.webgpu_resources.resize_surface(new_size);

        let compute_texture = build_texture(&self.webgpu_resources.device, self.size);
        let [compute_texture_bind_group, render_texture_bind_group] = texture_bind_groups(
            &self.webgpu_resources.device,
            &compute_texture,
            &self.sampler,
        );
        self.compute_texture_bind_group = compute_texture_bind_group;
        self.render_texture_bind_group = render_texture_bind_group;

        self.scene
            .camera
            .resize_viewport(&self.webgpu_resources.queue, self.size);
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
            compute_pass.set_bind_group(0, &self.compute_texture_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.camera_bind_group, &[]);
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
            render_pass.set_bind_group(0, &self.render_texture_bind_group, &[]);
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
