use std::ops::{Add, Div, Neg, Sub};

use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState, ColorTargetState, ColorWrites, ComputePipeline, ComputePipelineDescriptor, Device,
    Extent3d, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    ShaderStages, StorageTextureAccess, Texture, TextureDescriptor, TextureFormat, TextureUsages,
    TextureViewDescriptor, TextureViewDimension,
};
use winit::dpi::PhysicalSize;

use crate::camera::Camera;

#[derive(Default, Clone, Copy)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn i() -> Self {
        Vec3(1.0, 0.0, 0.0)
    }
    pub fn j() -> Self {
        Vec3(0.0, 1.0, 0.0)
    }
    pub fn k() -> Self {
        Vec3(0.0, 0.0, 1.0)
    }

    pub fn origin() -> Self {
        Vec3(0.0, 0.0, 0.0)
    }

    pub fn as_array(&self) -> [f32; 3] {
        [self.0, self.1, self.2]
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        Self(-self.0, -self.1, -self.2)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs, self.1 / rhs, self.2 / rhs)
    }
}

pub fn build_texture(device: &Device, size: PhysicalSize<u32>) -> Texture {
    device.create_texture(&TextureDescriptor {
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
    })
}

pub fn texture_bind_group_layouts(device: &Device) -> [BindGroupLayout; 2] {
    [
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    view_dimension: TextureViewDimension::D2,
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba8Unorm,
                },
                count: None,
            }],
        }),
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        }),
    ]
}

pub fn texture_bind_groups(
    device: &Device,
    texture: &Texture,
    sampler: &Sampler,
) -> [BindGroup; 2] {
    let view = texture.create_view(&TextureViewDescriptor::default());
    let layouts = texture_bind_group_layouts(device);
    [
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layouts[0],
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view),
            }],
        }),
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layouts[1],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        }),
    ]
}

pub fn build_compute_pipeline(device: &Device) -> ComputePipeline {
    let compute_shader = device.create_shader_module(include_wgsl!("compute.wgsl"));
    let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            &texture_bind_group_layouts(device)[0],
            &Camera::bind_group_layout(device),
        ],
        push_constant_ranges: &[],
    });
    device.create_compute_pipeline(&ComputePipelineDescriptor {
        module: &compute_shader,
        entry_point: "main",
        compilation_options: PipelineCompilationOptions::default(),
        label: None,
        layout: Some(&compute_pipeline_layout),
    })
}

pub fn build_render_pipeline(
    device: &Device,
    fragment_target_format: TextureFormat,
) -> RenderPipeline {
    let render_shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&texture_bind_group_layouts(device)[1]],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&RenderPipelineDescriptor {
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
                format: fragment_target_format,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::all(),
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    })
}
