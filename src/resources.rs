use std::{borrow::Cow, marker::PhantomData};

use bevy::{prelude::*, render::{render_resource::{BindGroup, BindGroupLayout, Buffer, CachedComputePipelineId, ComputePipelineDescriptor, OwnedBindingResource, PipelineCache, ShaderRef, StageBuffers}, renderer::RenderDevice}};

use crate::{ComputeTrait, Pass};

#[derive(Resource)]
pub struct PreparedCompute<T: ComputeTrait> {
    pub bindings: Vec<(u32, OwnedBindingResource)>,
    pub bind_group: BindGroup,
    pub staging_buffers: StageBuffers,
    pub staging_image_buffers: Vec<Buffer>,
    pub _marker: PhantomData<T>,
}


#[derive(Resource)]
pub struct RenderComputePasses<T: ComputeTrait> {
    pub passes: Vec<Pass>,
    pub images: Vec<(Handle<Image>, BufferDimensions)>,
    pub _marker: PhantomData<T>,
}

// nore a util, but used as a resource
#[derive(Copy, Clone)]
pub struct BufferDimensions {
    pub width: usize,
    pub height: usize,    
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}


/// Struct to manage data transfers from/to the GPU
#[derive(Resource)]
pub struct ComputePipeline<T: ComputeTrait> {
    // pipelines ordered by entry point
    pub pipelines: Vec<CachedComputePipelineId>,
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> FromWorld for ComputePipeline<T> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>().clone();
        let asset_server = world.resource::<AssetServer>();
        let shader = match T::shader() {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        }
        .unwrap();

        let bind_group_layout = T::bind_group_layout(&render_device);
        let pipeline_cache = world.resource::<PipelineCache>();

        let mut pipelines = Vec::new();
        for entry in T::entry_points() {
            let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: None,
                layout: vec![bind_group_layout.clone()], // S::layouts().to_vec(), use case?
                push_constant_ranges: T::push_constant_ranges().to_vec(),
                shader_defs: T::shader_defs().to_vec(),
                entry_point: Cow::Borrowed(entry),
                shader: shader.clone(), // TODO: how bad is this clone, could I use weak ref?
            });
            pipelines.push(pipeline);
        }

        Self {
            bind_group: None,
            bind_group_layout: bind_group_layout,
            pipelines,
            _marker: Default::default(),
        }
    }
}


impl BufferDimensions {
    pub fn new(width: usize, height: usize, bytes_per_pixel: usize) -> Self {
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = 256usize; // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}
