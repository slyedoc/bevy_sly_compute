use std::marker::PhantomData;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self},
        render_resource::{
            CachedPipelineState, ComputePassDescriptor, Extent3d, ImageCopyBuffer, ImageDataLayout,
            OwnedBindingResource, PipelineCache,
        },
        renderer::RenderContext,
    },
};

use crate::{ComputePipeline, ComputeTrait, PreparedCompute, RenderComputePasses};

// #[derive(Debug, Hash, Clone, RenderLabel)]
// pub struct ComputeLabel<T: ComputeTrait>(PhantomData<T>);

// impl<T: ComputeTrait> Default for ComputeLabel<T> {
//     fn default() -> Self {
//         Self(PhantomData)
//     }
// }

enum ComputeState {
    Loading,
    Ready,
}

pub struct ComputeNode<T: ComputeTrait> {
    state: ComputeState,
    _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputeNode<T> {
    fn default() -> Self {
        Self {
            state: ComputeState::Loading,
            _marker: Default::default(),
        }
    }
}

impl<T: ComputeTrait> render_graph::Node for ComputeNode<T> {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<ComputePipeline<T>>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            ComputeState::Loading => {
                // if all pipelines are ready, transition to the next stage
                if pipeline.pipelines.iter().all(|p| {
                    if let CachedPipelineState::Ok(_) =
                        pipeline_cache.get_compute_pipeline_state(*p)
                    {
                        true
                    } else {
                        false
                    }
                }) {
                    self.state = ComputeState::Ready;
                }
            }
            ComputeState::Ready => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> std::result::Result<(), bevy::render::render_graph::NodeRunError> {
        let Some(passes) = world.get_resource::<RenderComputePasses<T>>() else {
            // no compute events, exit
            // TODO: any better way to add node only when needed?
            return Ok(());
        };

        let prepaired = world.resource::<PreparedCompute<T>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let compute_pipelines = world.resource::<ComputePipeline<T>>();
        let gpu_images = world.resource::<RenderAssets<Image>>();

        let encoder = render_context.command_encoder();

        // // select the pipeline based on the current state
        match self.state {
            ComputeState::Loading => {}
            ComputeState::Ready => {
                // run multiple passes and dispatch workgroups
                // seemed like a simple solution, and appears to work
                for pass in passes.passes.iter() {
                    // get pipeline depending on entry point, need its index
                    let index = T::entry_points()
                        .iter()
                        .position(|&x| x == pass.entry)
                        .unwrap_or(0);
                    let Some(pipeline) =
                        pipeline_cache.get_compute_pipeline(compute_pipelines.pipelines[index])
                    else {
                        return Ok(());
                    };

                    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some(pass.entry),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, &prepaired.bind_group, &[]);
                    for workgroup in pass.workgroups.iter() {
                        cpass.dispatch_workgroups(workgroup.x, workgroup.y, workgroup.z);
                    }
                }

                // copy gpu buffer to staging buffer on cpu for storage
                for (index, staging_buff) in prepaired.staging_buffers.storage.iter() {
                    // find resource on gpu
                    if let Some((_i, OwnedBindingResource::Buffer(gpu_buffer))) =
                        prepaired.bindings.iter().find(|(i, _)| i == index)
                    {
                        encoder.copy_buffer_to_buffer(
                            &gpu_buffer,
                            0,
                            &staging_buff,
                            0,
                            gpu_buffer.size(),
                        );
                    } else {
                        error!("failed to find binding resource for staging");
                    }
                }

                // copy gpu texture to staging buffer on cpu
                for (index, (handle, dim)) in passes.images.iter().enumerate() {
                    let buffer = &prepaired.staging_image_buffers[index];
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
                }
            }
        }

        Ok(())
    }
}
