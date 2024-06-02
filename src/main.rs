use std::sync::Arc;

use wgpu::{
    Device, DeviceDescriptor, Instance, InstanceDescriptor, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, SurfaceError, TextureUsages,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

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
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);

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

struct App<'a> {
    window: Arc<Window>,
    size: PhysicalSize<u32>,
    webgpu_resources: WebGPUResources<'a>,
}

impl<'a> App<'a> {
    fn new(window: Window) -> Self {
        let window = Arc::new(window);
        Self {
            size: window.inner_size(),
            webgpu_resources: WebGPUResources::new(window.clone()),
            window,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.webgpu_resources.resize_surface(new_size);
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), SurfaceError> {
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
