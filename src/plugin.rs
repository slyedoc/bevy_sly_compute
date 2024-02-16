#![allow(unused_imports)]
use core::panic;
use std::{borrow::Cow, marker::PhantomData, ops::Deref};

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

        app.add_state::<WorkerState>()
            .add_event::<ComputeEvent<T>>()
            .add_systems(Startup, setup::<T>)
            .add_systems(
                Update,
                listen::<T>
                    .run_if(in_state(WorkerState::Available))
                    .run_if(on_event::<ComputeEvent<T>>()),
            )
            .add_systems(PostUpdate, run::<T>.run_if(in_state(WorkerState::Working)));
    }
}

// TODO: how do I make this generic, or add to macro?
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum WorkerState {
    //Created,
    #[default]
    Available,
    Working,
}

#[derive(Event)]
pub struct ComputeEvent<T: ComputeTrait> {
    pub workgroups: [u32; 3],
    pub vars: Vec<String>,
    pub _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeEvent<T> {
    fn default() -> Self {
        ComputeEvent::<T> {
            workgroups: [1, 1, 1],
            vars: vec![],
            _marker: Default::default(),
        }
    }
}

fn setup<T: ComputeTrait>(
    mut commands: Commands,
) {
    // delay the creation till pipeline cache exists
    commands.init_resource::<ComputeWorker<T>>();
}

fn listen<T: ComputeTrait>(
    mut events: EventReader<ComputeEvent<T>>,
    mut worker: ResMut<ComputeWorker<T>>,
    mut next_state: ResMut<NextState<WorkerState>>,
) {
    events.read().for_each(|event| {
        if event.workgroups[0] == 0 || event.workgroups[1] == 0 || event.workgroups[2] == 0 {
            warn!("invalid workgroups for compute event, skipping");
            return;
        }
        // queue up the work
        worker.steps = vec![ComputePass {
            workgroups: event.workgroups,
            vars: event.vars.clone(),
        }];
        next_state.set(WorkerState::Working);
    });
}

fn run<T: ComputeTrait>(
    mut data: ResMut<T>,
    worker: Res<ComputeWorker<T>>,
    pipeline_cache: Res<AppPipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<ComputeAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    mut next_state: ResMut<NextState<WorkerState>>,
) {
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
        render_device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    // create staging buffers
    let staging = data.create_staging_buffers(&render_device);

    let pipeline = pipeline_cache
        .get_compute_pipeline(worker.main_pipeline)
        .expect("pipeline not found");

    for pass in worker.steps.iter() {
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
        cpass.set_pipeline(pipeline);
        cpass.set_bind_group(0, &prepared.bind_group, &[]);
        cpass.dispatch_workgroups(pass.workgroups[0], pass.workgroups[1], pass.workgroups[2]);
        //cpass.insert_debug_marker("compute");
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

    //drop(buffer_slices);

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

    // Returns data from buffer
    next_state.set(WorkerState::Available);
}

#[derive(Default, Clone, Debug)]
pub struct ComputePass {
    pub workgroups: [u32; 3],
    pub vars: Vec<String>,
}

/// Struct to manage data transfers from/to the GPU
/// it also handles the logic of your compute work.
#[derive(Resource)]
pub struct ComputeWorker<T: ComputeTrait> {
    // Pipeline info
    pub main_pipeline: CachedAppComputePipelineId,

    // bindgroup layout
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,

    pub steps: Vec<ComputePass>,

    pub marker: PhantomData<T>,
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

        let main_pipeline = pipeline_cache.queue_app_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()], // S::layouts().to_vec(), use case?
            push_constant_ranges: T::push_constant_ranges().to_vec(),
            shader_defs: T::shader_defs().to_vec(),
            entry_point: Cow::Borrowed(T::entry_point()),
            shader,
        });

        Self {
            bind_group: None,
            bind_group_layout: bind_group_layout,
            main_pipeline,
            steps: vec![],
            marker: Default::default(),
        }
    }
}
