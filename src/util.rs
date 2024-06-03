use std::ops::Neg;

use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BlendState, Buffer, ColorTargetState,
    ColorWrites, ComputePipeline, ComputePipelineDescriptor, Device, Extent3d, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, Sampler, ShaderStages, Texture, TextureDescriptor, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

#[derive(Default)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub fn origin() -> Self {
        Self::default()
    }
}

#[derive(Default)]
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
}

impl Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        Self(-self.0, -self.1, -self.2)
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

pub struct ComputeBindGroupBuilder(BindGroupLayout);
impl ComputeBindGroupBuilder {
    pub fn new(device: &Device) -> Self {
        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
        Self(compute_bind_group_layout)
    }

    pub fn build(
        &self,
        device: &Device,
        compute_texture: &Texture,
        resolution_uniform: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.0,
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
        })
    }
}

pub fn build_compute_pipeline(
    device: &Device,
    compute_bind_group_builder: &ComputeBindGroupBuilder,
) -> ComputePipeline {
    let compute_shader = device.create_shader_module(include_wgsl!("compute.wgsl"));
    let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&compute_bind_group_builder.0],
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

pub struct RenderBindGroupBuilder(BindGroupLayout);
impl RenderBindGroupBuilder {
    pub fn new(device: &Device) -> Self {
        let render_bind_group_layout =
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
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        Self(render_bind_group_layout)
    }

    pub fn build(&self, device: &Device, texture: &Texture, sampler: &Sampler) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.0,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

pub fn build_render_pipeline(
    device: &Device,
    render_bind_group_builder: &RenderBindGroupBuilder,
    fragment_target_format: TextureFormat,
) -> RenderPipeline {
    let render_shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&render_bind_group_builder.0],
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
