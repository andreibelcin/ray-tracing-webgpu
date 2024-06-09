use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferUsages, Device, Queue, ShaderStages,
};
use winit::dpi::PhysicalSize;

use crate::util::Vec3;

pub struct Camera {
    pub origin: Vec3,
    pub viewport: Viewport,
    origin_buffer: Buffer,
    viewport_buffers: [Buffer; 2],
    pixel_00_center: Vec3,
    pixel_buffer: Buffer,
}

impl Camera {
    pub fn new(image_size: PhysicalSize<u32>, device: &Device) -> Self {
        let origin = Vec3::origin();
        let origin_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&origin.as_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let viewport = Viewport::new(image_size);
        let viewport_buffers = [
            device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&viewport.du.as_array()),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
            device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&viewport.dv.as_array()),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            }),
        ];

        let upper_corner =
            origin - Vec3(0.0, 0.0, viewport.focal_len) - (viewport.u / 2.0) - (viewport.v / 2.0);
        let pixel_00_center = upper_corner + (viewport.du + viewport.dv) / 2.0;
        let pixel_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&pixel_00_center.as_array()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        Self {
            origin,
            viewport,
            origin_buffer,
            viewport_buffers,
            pixel_00_center,
            pixel_buffer,
        }
    }

    pub fn bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn bind_group(&self, device: &Device) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &Self::bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(
                        self.origin_buffer.as_entire_buffer_binding(),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(
                        self.viewport_buffers[0].as_entire_buffer_binding(),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(
                        self.viewport_buffers[1].as_entire_buffer_binding(),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Buffer(self.pixel_buffer.as_entire_buffer_binding()),
                },
            ],
        })
    }

    pub fn resize_viewport(&mut self, queue: &Queue, size: PhysicalSize<u32>) {
        self.viewport.resize(size);
        queue.write_buffer(
            &self.viewport_buffers[0],
            0,
            bytemuck::cast_slice(&self.viewport.du.as_array()),
        );
        queue.write_buffer(
            &self.viewport_buffers[1],
            0,
            bytemuck::cast_slice(&self.viewport.dv.as_array()),
        );

        self.update_pixel_buffer(queue);
    }

    fn update_pixel_buffer(&mut self, queue: &Queue) {
        let upper_corner = self.origin
            - Vec3(0.0, 0.0, self.viewport.focal_len)
            - (self.viewport.u / 2.0)
            - (self.viewport.v / 2.0);
        self.pixel_00_center = upper_corner + (self.viewport.du + self.viewport.dv) / 2.0;
        queue.write_buffer(
            &self.pixel_buffer,
            0,
            bytemuck::cast_slice(&self.pixel_00_center.as_array()),
        );
    }
}

pub struct Viewport {
    width: f32,
    height: f32,
    focal_len: f32,
    u: Vec3,
    v: Vec3,
    du: Vec3,
    dv: Vec3,
}

impl Viewport {
    pub fn new(image_size: PhysicalSize<u32>) -> Self {
        let height = 2.0;
        let width = height * (image_size.width as f32 / image_size.height as f32);

        let u = Vec3(width, 0.0, 0.0);
        let v = Vec3(0.0, -height, 0.0);

        let du = u / image_size.width as _;
        let dv = v / image_size.height as _;

        Self {
            height,
            width,
            focal_len: 1.0,
            u,
            v,
            du,
            dv,
        }
    }

    pub fn with_focal_len(mut self, focal_len: f32) -> Self {
        self.focal_len = focal_len;
        self
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.width = self.height * (size.width as f32 / size.height as f32);
        self.u = Vec3(self.width, 0.0, 0.0);

        self.du = self.u / size.width as _;
        self.dv = self.v / size.height as _;
    }
}
