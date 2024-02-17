#![allow(unused_imports)]
use core::panic;
use std::{borrow::Cow, marker::PhantomData, ops::Deref, vec};

use bevy::{
    ecs::world::{FromWorld, World},
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupLayout, Buffer, ComputePipeline, ComputePipelineDescriptor,
            OwnedBindingResource, ShaderRef,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::FallbackImage,
    },
    utils::{HashMap, Uuid},
};
use bytemuck::{bytes_of, cast_slice, from_bytes, AnyBitPattern, NoUninit};
use wgpu::{BindGroupEntry, CommandEncoder, CommandEncoderDescriptor, ComputePassDescriptor};

use crate::{
    error::{Error, Result},
    pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId},
    ComputeAssets, ComputePlugin, ComputeShader, ComputeTrait,
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

        app.add_event::<ComputeEvent<T>>()
            .add_systems(Startup, setup::<T>)
            .add_systems(
                Update, run::<T>.run_if(on_event::<ComputeEvent<T>>()),
            );
    }
}

#[derive(Event)]
pub struct ComputeEvent<T: ComputeTrait> {
    pub passes: Vec<ComputePass>,
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeEvent<T> {
    fn default() -> Self {
        Self { 
            passes: vec![ComputePass {
                entry: T::entry_points().first().expect("no entry points"),
                workgroups: vec![UVec3::new(1, 1, 1)],
            }], 
            _marker: Default::default()
         }
    }
}

impl<T: ComputeTrait> ComputeEvent<T> {
    pub fn new(workgroups: UVec3) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                ComputePass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![workgroups],
                }
            ],
            _marker: Default::default(),
        }
    }

    pub fn new_xyz( x: u32, y: u32, z: u32) -> Self {        
        ComputeEvent::<T> {
            passes: vec![
                ComputePass {
                    entry: T::entry_points().first().expect("no entry points"),
                    workgroups: vec![UVec3::new(x, y, z)],
                }
            ],
            _marker: Default::default(),
        }
    }

    pub fn add_pass(&mut self, entry: &'static str, workgroup: UVec3) -> &mut Self {
        self.passes.push(ComputePass::new(entry, workgroup));
        self
    }

}

fn setup<T: ComputeTrait>(mut commands: Commands) {
    // delay the creation till pipeline cache exists
    commands.init_resource::<ComputeWorker<T>>();
}

fn run<T: ComputeTrait>(
    mut events: EventReader<ComputeEvent<T>>,
    worker: Res<ComputeWorker<T>>, 
    mut data: ResMut<T>,
    pipeline_cache: Res<AppPipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<ComputeAssets<Image>>,
    fallback_image: Res<FallbackImage>,
) {
    events.read().for_each(|event| {

        info!("running compute event");
        event.passes.iter().for_each(|pass| {
            // check entry points
            if !T::entry_points().contains(&pass.entry) {
                warn!("invalid entry point for compute event {:?}, skipping", pass);
                return;
            }

            // check workgroup sizes            
            pass.workgroups.iter().for_each(|workgroup| {
                if workgroup.x == 0 || workgroup.y == 0 || workgroup.z == 0 {
                    warn!("invalid workgroups for compute event {:?}, skipping", pass);
                    return;
                }
            });

        });
        
        // Ordering seems off here, are gpu images ready?
        // NOTE: This would be in a prepare function if we used a different compute world
        let Ok(prepared) = data.as_bind_group(
            &worker.bind_group_layout,
            &render_device,
            &gpu_images,
            &fallback_image,
        ) else {
            error!("failed to prepare compute worker bind group");
            return;
        };

        let mut encoder =
            render_device.create_command_encoder(&CommandEncoderDescriptor { label: T::label() });

        // create staging buffers,
        let staging = data.create_staging_buffers(&render_device);

        // run passes
        for pass in event.passes.iter() {
            
            // get pipeline depending on entry point, need index
            let Some(index) = T::entry_points().iter().position(|&x| x == pass.entry) else {
                error!("failed to find entry point {} for compute event", pass.entry);
                return;
            };    
            
            let Some(pipeline) = pipeline_cache.get_compute_pipeline(worker.pipelines[index]) else {
                error!("failed to find pipeline for compute event");
                return;
            };
        
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
            cpass.set_pipeline(pipeline);
            cpass.set_bind_group(0, &prepared.bind_group, &[]);
            for workgroup in pass.workgroups.iter() {
                cpass.dispatch_workgroups(workgroup.x, workgroup.y, workgroup.z);
            }                        
        }

        // copy buffer to staging
        //encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, size);
        for (index, staging_buffer) in staging.iter() {
            let (_, buffer) = prepared.bindings.iter().find(|(i, _)| i == index).unwrap();
            let OwnedBindingResource::Buffer(buffer) = buffer else {
                error!("failed to find buffer for staging");
                return;
            };
            encoder.copy_buffer_to_buffer(&buffer, 0, &staging_buffer, 0, buffer.size());
        }

        // submit
        render_queue.submit(Some(encoder.finish()));

        // map buffer

        // let buffer_slice = staging_buffer.slice(..);
        // buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        //     let err = result.err();
        //     if err.is_some() {
        //         let some_err = err.unwrap();
        //         panic!("{}", some_err.to_string());
        //     }
        // });
        let buffer_slices = staging
            .iter()
            .map(|(index, staging_buffer)| {
                let buffer_slice = staging_buffer.slice(..);
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

        render_device.wgpu_device().poll(wgpu::MaintainBase::Wait);

        // let data = buffer_slice.get_mapped_range();
        // // Since contents are got in bytes, this converts these bytes back to f32
        // let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        // dbg!(result);

        // Write the data to T

        data.map_staging_mappings(&buffer_slices);

        drop(buffer_slices);

        staging.iter().for_each(|(_, staging_buffer)| {
            staging_buffer.unmap();
        });

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        //drop(data);

        // staging_buffer.unmap(); // Unmaps buffer from memory
        //                         // If you are familiar with C++ these 2 lines can be thought of similarly to:
        //                         //   delete myPointer;
        //                         //   myPointer = NULL;
        //                         // It effectively frees the memory
    });
}
    
#[derive(Clone, Debug)]
pub struct ComputePass {
    /// entry point for pipeline, need pipeline per entry point
    pub entry: &'static str,

    /// workgroup sizes to run for entry point
    pub workgroups: Vec<UVec3>,
}

impl ComputePass {
    pub fn new(entry: &'static str, workgroups: UVec3) -> Self {
        ComputePass {
            entry,
            workgroups: vec![workgroups],
        }
    }
}

/// Struct to manage data transfers from/to the GPU
/// it also handles the logic of your compute work.
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
