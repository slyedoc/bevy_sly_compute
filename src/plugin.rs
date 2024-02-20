#![allow(unused_imports)]
use core::panic;
use std::{borrow::Cow, default, marker::PhantomData, ops::Deref, vec};

use bevy::{
    ecs::world::{FromWorld, World},
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::{FallbackImage, TextureFormatPixelInfo},
        view::screenshot::layout_data,
    },
    ui::debug,
    utils::{HashMap, Uuid},
};

use bytemuck::{bytes_of, cast_slice, from_bytes, AnyBitPattern, NoUninit};
use wgpu::{BindGroupEntry, CommandEncoder, CommandEncoderDescriptor, ComputePassDescriptor};

use crate::{
    error::{Error, Result}, events::*, pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId}, ComputeAssets, ComputeEvent, ComputePlugin, ComputeShader, ComputeTrait, RequeueComputeEvent, StageBindGroup
};

use crate::AsBindGroupCompute;

pub struct ComputeWorkerPlugin<T: ComputeTrait> {
    _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeWorkerPlugin<T> {
    fn default() -> Self {
        ComputeWorkerPlugin {
            _marker: PhantomData,
        }
    }
}

impl<T: ComputeTrait> Plugin for ComputeWorkerPlugin<T> {
    fn build(&self, app: &mut App) {
        // instead of having everyone add the plugin, we add when it's not there
        if !app.is_plugin_added::<ComputePlugin>() {
            app.add_plugins(ComputePlugin::default());
        }

        app
            .add_event::<ComputeEvent<T>>()
            .add_event::<crate::events::ComputeComplete<T>>()
            .add_event::<RequeueComputeEvent<T>>()
            .add_systems(Startup, setup::<T>)
            // all work is done here
            .add_systems(Update, 
                (   
                    run::<T>,
                    // hack to retry when pipeline is not ready
                    requeue::<T>
                ).chain()
                .run_if(on_event::<ComputeEvent<T>>())
            );
    }
}


fn setup<T: ComputeTrait>(mut commands: Commands) {
    // delay the creation till pipeline cache exists
    commands.init_resource::<ComputeWorker<T>>();
}

// NOTE:  All gpu work is done here, this could be split into multiple systems,
//  but since I am learning this its been much easyer to keep it all in one place to reason on
fn run<T: ComputeTrait>(
    mut events: EventReader<ComputeEvent<T>>,
    mut requeue_events: EventWriter<RequeueComputeEvent<T>>,
    mut complete_events: EventWriter<ComputeComplete<T>>,
    mut asset_event: EventWriter<AssetEvent<Image>>,

    worker: Res<ComputeWorker<T>>,
    mut data: ResMut<T>,
    pipeline_cache: Res<AppPipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut images: ResMut<Assets<Image>>,
    gpu_images: Res<ComputeAssets<Image>>,
    fallback_image: Res<FallbackImage>,
) {
    // TODO: test multiple events
    events.read().for_each(|event| {
        // check passes for early out
        event.passes.iter().for_each(|pass| {
            if !T::entry_points().contains(&pass.entry) {
                warn!("invalid entry point for compute event {:?}, skipping", pass);
                return;
            }
            pass.workgroups.iter().for_each(|workgroup| {
                if workgroup.x == 0 || workgroup.y == 0 || workgroup.z == 0 {
                    warn!("invalid workgroups for compute event {:?}, skipping", pass);
                    return;
                }
            });
        });

        // Generate bind group
        let Ok(prepared) = data.as_bind_group(
            &worker.bind_group_layout,
            &render_device,
            &gpu_images,
            &fallback_image,
        ) else {
            if event.retry > 3 {
                error!("failed to prepare compute worker bind group, retry limit reached");
            }
            else {
                requeue_events.send(RequeueComputeEvent {
                    passes: event.passes.clone(),
                    retry: event.retry + 1,
                    _marker: Default::default(),
                });
            }
            return;
        };

        // create staging buffers
        let StageBindGroup {
            storage: storage_buffers,
            handles: stage_image,
        } = data.create_staging_buffers(&render_device);

        // create staging buffers for images
        // NOTE: It is a WebGPU requirement that ImageCopyBuffer.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
        // So we calculate padded_bytes_per_row by rounding unpadded_bytes_per_row
        // up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
        // https://en.wikipedia.org/wiki/Data_structure_alignment#Computing_padding
        let stage_image = stage_image
            .into_iter()
            .map(|handle| {
                let image = images.get(&handle).unwrap();
                //let _gpu_image = gpu_images.get(handle).unwrap();
                let width = image.width() as usize;
                let height = image.height() as usize;
                let pixel_size = image.texture_descriptor.format.pixel_size();
                let buffer_dimensions = BufferDimensions::new(width, height, pixel_size as usize);
                (
                    handle,
                    render_device.create_buffer(&bevy::render::render_resource::BufferDescriptor {
                        label: None,
                        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                        size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height)
                            as u64,
                        mapped_at_creation: false,
                    }),
                    buffer_dimensions,
                )
            })
            .collect::<Vec<_>>();

        // create command encoder for our compute passes
        let mut encoder =
            render_device.create_command_encoder(&CommandEncoderDescriptor { label: T::label() });

        // run multiple passes and dispatch workgroups
        // seemed like a simple solution, and appears to work
        for pass in event.passes.iter() {
            // get pipeline depending on entry point, need its index
            let Some(index) = T::entry_points().iter().position(|&x| x == pass.entry) else {
                error!(
                    "failed to find entry point {} for compute event",
                    pass.entry
                );
                return;
            };

            let Some(pipeline) = pipeline_cache.get_compute_pipeline(worker.pipelines[index])
            else {
                error!("failed to find pipeline for compute event");
                return;
            };

            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(pass.entry),
            });
            cpass.set_pipeline(pipeline);
            cpass.set_bind_group(0, &prepared.bind_group, &[]);
            for workgroup in pass.workgroups.iter() {
                cpass.dispatch_workgroups(workgroup.x, workgroup.y, workgroup.z);
            }
        }

        // copy resource from gpu to buffer on cpu
        for (index, staging_buff) in storage_buffers.iter() {
            // find resource on gpu
            let Some((_, OwnedBindingResource::Buffer(gpu_buffer))) =
                prepared.bindings.iter().find(|(i, _)| i == index)
            else {
                error!("failed to find binding resource for staging");
                return;
            };

            encoder.copy_buffer_to_buffer(&gpu_buffer, 0, &staging_buff, 0, gpu_buffer.size());
        }

        // copy images from gpu to buffer on cpu
        stage_image
            .iter()
            .for_each(|(handle, buffer, dim)| {
                let gpu_image = gpu_images.get(handle).unwrap();
                encoder.copy_texture_to_buffer(
                    gpu_image.texture.as_image_copy(),
                    ImageCopyBuffer {
                        buffer: &buffer,
                        layout: ImageDataLayout {
                            bytes_per_row: Some(dim.padded_bytes_per_row as u32),
                            rows_per_image: None,
                            ..Default::default()
                        },
                    },
                    Extent3d {
                        width: dim.width as u32,
                        height: dim.height as u32,
                        depth_or_array_layers: 1,
                    },
                );
            });

        // submit, this will wait for everything to finish,
        // TODO: we have no idea what render app is doing
        render_queue.submit(Some(encoder.finish()));

        // Note that we're not calling `.await` here will call render_device.wgpu_device().poll belo
        let storage_buffer_slices = storage_buffers
            .iter()
            .map(|(index, buffer)| {
                let buffer_slice = buffer.slice(..);
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    let err = result.err();
                    if err.is_some() {
                        let some_err = err.unwrap();
                        panic!("{}", some_err.to_string());
                    }
                });
                (*index, buffer_slice)
            })
            .collect::<Vec<_>>();

        let image_buffer_slices = stage_image.iter()            
            .map(|(_handle, buffer, _dim)| {
                let buffer_slice = buffer.slice(..);
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    let err = result.err();
                    if err.is_some() {
                        let some_err = err.unwrap();
                        panic!("{}", some_err.to_string());
                    }
                });
                buffer_slice
            })
            .collect::<Vec<_>>();

        // await here?

        // wait for gpu to finish
        render_device.wgpu_device().poll(wgpu::MaintainBase::Wait);

        // Write the data from buffer slices back to T
        // Will trigger change detection
        data.bypass_change_detection()
            .map_storage_mappings(&storage_buffer_slices);

        for (index, (handle, _buffer, dim)) in stage_image.iter().enumerate() {
            let image = images.get_mut(handle).unwrap();
            let pixel_size = image.texture_descriptor.format.pixel_size() as usize; // Assuming you have this

            let padded_data = &image_buffer_slices[index].get_mapped_range();

            // I need to convert the padded buffer to unpadded buffer
            let mut unpadded_data = Vec::with_capacity(dim.width * dim.height * pixel_size);
            for row in 0..dim.height {
                let start = row * dim.padded_bytes_per_row;
                let end = start + dim.unpadded_bytes_per_row;
                unpadded_data.extend_from_slice(&padded_data[start..end]);
            }

            // this doesnt work
            image.data = unpadded_data;

            // notify asset event
            asset_event.send(AssetEvent::Modified {
                id: handle.id(),
            });
        }

        drop(storage_buffer_slices);
        drop(image_buffer_slices);

        storage_buffers.iter().for_each(|(_, buffer)| {
            buffer.unmap();
        });
        stage_image.iter().for_each(|(_, buffer, _)| {
            buffer.unmap();
        });

        complete_events.send(ComputeComplete::<T>::default());
    });
}

fn requeue<T: ComputeTrait>(
    mut events: EventReader<RequeueComputeEvent<T>>,
    mut compute: EventWriter<ComputeEvent<T>>,
) {
    
    events.read().for_each(|event| {
        compute.send(ComputeEvent::<T> {
            passes: event.passes.clone(),
            ..default()
        });
    });
}

/// Struct to manage data transfers from/to the GPU
#[derive(Resource)]
pub struct ComputeWorker<T: ComputeTrait> {
    // pipelines ordered by entry point
    pub pipelines: Vec<CachedAppComputePipelineId>,

    // bind group
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,

    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> FromWorld for ComputeWorker<T> {
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
        let pipeline_cache = world.resource_mut::<AppPipelineCache>();

        let mut pipelines = Vec::new();
        for entry in T::entry_points() {
            let pipeline = pipeline_cache.queue_app_compute_pipeline(ComputePipelineDescriptor {
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

struct BufferDimensions {
    width: usize,
    height: usize,
    unpadded_bytes_per_row: usize,
    padded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize, bytes_per_pixel: usize) -> Self {
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
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
