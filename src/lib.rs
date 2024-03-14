#![feature(iter_collect_into)]

mod traits;
use std::marker::PhantomData;

use channel::{create_compute_channels, ComputeMessage, ComputeReceiver, ComputeSender};
pub use traits::*;

mod node;
pub use node::*;

mod resources;
pub use resources::*;

mod events;
pub use events::*;

mod channel;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::RenderGraph,
        render_resource::{BufferDescriptor, BufferUsages, Maintain, MapMode},
        renderer::RenderDevice,
        texture::{FallbackImage, TextureFormatPixelInfo},
        Extract, Render, RenderApp, RenderSet,
    },
};

/// Helper module to import most used elements.
pub mod prelude {
    pub use crate::{
        events::{Pass, *},
        mark_shader_modified,
        node::*,
        traits::*,
        ComputePlugin, MainComputePlugin,
    };
    // Since these are always used when using this crate
    pub use bevy::render::render_resource::{ShaderRef, ShaderType};
}

/// A global plugin, doesn't do much currently
pub struct MainComputePlugin;

impl Plugin for MainComputePlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<Self>() {
            return;
        }

        // HACK: update StandardMaterial when any images are modified
        // currently have no way to update a material that shares an image
        // this is a workaround to mark all materials as modified
        app.add_systems(
            First,
            (mark_shader_modified::<StandardMaterial>,).run_if(on_event::<AssetEvent<Image>>()),
        );
    }
}

pub struct ComputePlugin<T: ComputeTrait> {
    _marker: PhantomData<T>,
}

impl<T: ComputeTrait> Default for ComputePlugin<T> {
    fn default() -> Self {
        ComputePlugin {
            _marker: PhantomData,
        }
    }
}

impl<T: ComputeTrait> Plugin for ComputePlugin<T> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<MainComputePlugin>() {
            app.add_plugins(MainComputePlugin);
        }

        // we need some way to safely send data from main app from render app
        let (sender, receiver) = create_compute_channels::<T>();

        app.insert_resource(receiver)
            .add_event::<ComputeEvent<T>>()
            .add_event::<ComputeComplete<T>>()
            .add_systems(Last, listen_receiver::<T>.run_if(resource_exists::<T>))
            // build event for shader modified
            .add_event::<ComputeShaderModified<T>>()
            .add_systems(Update, events::shader_modified::<T>);

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .insert_resource(sender)
            // checks for compute events and extracts the main resource into the render world
            // also grabs image handles and dimensions for later use
            .add_systems(ExtractSchedule, extract_resource::<T>)
            .add_systems(
                Render,
                (prepare_bind_group::<T>)
                    .run_if(resource_exists::<RenderComputePasses<T>>)
                    .in_set(RenderSet::PrepareBindGroups),
            )
            .add_systems(
                Render,
                (
                    // the reads staging buffers and sends the data to the app world
                    read_and_send::<T>
                        .run_if(resource_exists::<RenderComputePasses<T>>)
                        .in_set(RenderSet::Cleanup),
                ),
            );

        // add nodes to render graph

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        T::set_nodes(&mut render_graph);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ComputePipeline<T>>();
    }
}

// Hack to mark asset modified so they will noice there Handle<Image> have been modified
pub fn mark_shader_modified<R: Asset>(mut assets: ResMut<Assets<R>>) {
    let ids = assets.ids().collect::<Vec<_>>();
    for id in ids {
        assets.queued_events.push(AssetEvent::Modified { id });
    }
}

fn listen_receiver<T: ComputeTrait>(
    mut data: ResMut<T>,
    receiver: Res<ComputeReceiver<T>>,
    //mut has_received_time: Local<bool>,
    mut complete_events: EventWriter<ComputeComplete<T>>,
    mut asset_event: EventWriter<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Ok(msg) = receiver.try_recv() {
        // So this is a bit of a hack, most of the time its image data changing, and we dont know if what on T has changed nice our copy
        // was taken, so dont over write it if you dont have to
        if let Some(d) = msg.data {
            *data.bypass_change_detection() = d;
        }

        // update images
        for (handle, image_data) in msg.images {
            let image = images.get_mut(&handle).unwrap();
            image.data = image_data;
            asset_event.send(AssetEvent::Modified { id: handle.id() });
        }
        complete_events.send(ComputeComplete::<T>::default());
    }
}

// Based on ExtractResourcePlugin::<T>::default(), but we only want extract when we have a ComputeEvent
// Also extract passes from ComputeEvent into RenderComputePasses, and grab image handles and dimensions
// TODO: make a new trait? We are doing more work in extract than we should
pub fn extract_resource<T: ComputeTrait>(
    mut commands: Commands,
    mut compute_events: Extract<EventReader<ComputeEvent<T>>>,
    main_resource: Extract<Option<Res<T::Source>>>,
    target_resource: Option<ResMut<T>>,
    images: Extract<Res<Assets<Image>>>,
    mut passes: Local<Vec<Pass>>,
    mut passes_used: Local<Vec<&'static str>>,
) {
    passes.clear();
    passes_used.clear();

    // extract main resource
    let Some(main_resource) = main_resource.as_ref() else {
        warn_once!("no main resource for compute event");
        return;
    };

    // check passes are valid
    let mut passes = compute_events
        .read()
        .flat_map(|event| event.passes.iter().cloned())
        .filter(|pass| {
            let mut valid = true;
            if !T::entry_points().contains(&pass.entry) {
                warn!("invalid entry point for compute event {:?}, skipping", pass);
                valid = false;
            }
            pass.workgroups.iter().for_each(|workgroup| {
                if workgroup.x == 0 || workgroup.y == 0 || workgroup.z == 0 {
                    warn!("invalid workgroups for compute event {:?}, skipping", pass);
                    valid = false;
                }
            });
            valid
        })
        .collect::<Vec<_>>();

    // remove duplicates events
    passes.retain(|p| {
        if !passes_used.contains(&p.entry) {
            passes_used.push(p.entry);
            true
        } else {
            false
        }
    });

    // nothing to do, exit
    // TODO: do we need to remove resource?
    if passes.is_empty() {
        //commands.remove_resource::<T>();
        commands.remove_resource::<RenderComputePasses<T>>();
        return;
    }

 
    // extract render world version, and get list of image data
    // TODO: I would love to reuse the image.data, but I dont have access to it here,
    // so creating new vec and sending it back
    let images_handles = if let Some(mut target_resource) = target_resource {
        *target_resource = T::extract_resource(main_resource);
        T::image_handles(&target_resource)
    } else {
        let resource = T::extract_resource(main_resource);
        let image_handles = T::image_handles(&resource);
        commands.insert_resource(T::extract_resource(main_resource));
        image_handles
    };

    // we need a bit more information about any images while we can still acces them
    // since buffer dimensions can differ
    let image_info = images_handles
        .into_iter()
        .map(|handle| {
            let image = images.get(&handle).unwrap();
            let buffer_dimensions = BufferDimensions::new(
                image.width() as usize,
                image.height() as usize,
                image.texture_descriptor.format.pixel_size(),
            );
            (handle, buffer_dimensions)
        })
        .collect::<Vec<_>>();

    commands.insert_resource(RenderComputePasses::<T> {
        passes,
        images: image_info,
        _marker: Default::default(),
    });
}

fn prepare_bind_group<T: ComputeTrait>(
    mut commands: Commands,
    pipeline: Res<ComputePipeline<T>>,
    gpu_images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    data: Res<T>,
    render_device: Res<RenderDevice>,
    render_compute_passes: Res<RenderComputePasses<T>>,
) {
    // Generate normal bind group
    let Ok(prepared) = data.as_bind_group(
        &pipeline.bind_group_layout,
        &render_device,
        &gpu_images,
        &fallback_image,
    ) else {
        error!("error preparing bind group for compute event");
        return;
    };

    // get staging buffers, without images
    let staging_buffers = data.create_staging_buffers(&render_device);

    // create staging buffers for images
    // NOTE: It is a WebGPU requirement that ImageCopyBuffer.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
    // So we calculate padded_bytes_per_row by rounding unpadded_bytes_per_row
    // up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
    // https://en.wikipedia.org/wiki/Data_structure_alignment#Computing_padding
    let staging_image_buffers = render_compute_passes
        .images
        .iter()
        .map(|(_handle, dim)| {
            render_device.create_buffer(&BufferDescriptor {
                label: None,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                size: (dim.padded_bytes_per_row * dim.height) as u64,
                mapped_at_creation: false,
            })
        })
        .collect::<Vec<_>>();

    commands.insert_resource(PreparedCompute::<T> {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        staging_image_buffers: staging_image_buffers,
        staging_buffers,
        _marker: Default::default(),
    });
}

// This reads the staging buffers and sends the data to the app world
fn read_and_send<T: ComputeTrait>(
    mut commands: Commands,
    mut data: ResMut<T>,
    prepared: Res<PreparedCompute<T>>,
    render_compute_passes: ResMut<RenderComputePasses<T>>,
    sender: Res<ComputeSender<T>>,
    render_device: Res<RenderDevice>,
) {      
    // create buffer slices for storage buffers and images
    let storage_buffer_slices = prepared
        .staging_buffers
        .storage
        .iter()
        .map(|(index, buffer)| {
            let buffer_slice = buffer.slice(..);
            buffer_slice.map_async(MapMode::Read, move |result| {
                let err = result.err();
                if err.is_some() {
                    let some_err = err.unwrap();
                    panic!("{}", some_err.to_string());
                }
            });
            (*index, buffer_slice)
        })
        .collect::<Vec<_>>();

    let image_buffer_slices = prepared
        .staging_image_buffers
        .iter()
        .map(|buffer| {
            let buffer_slice = buffer.slice(..);
            buffer_slice.map_async(MapMode::Read, move |result| {
                let err = result.err();
                if err.is_some() {
                    let some_err = err.unwrap();
                    panic!("{}", some_err.to_string());
                }
            });
            buffer_slice
        })
        .collect::<Vec<_>>();

    // wait for gpu to finish
    render_device.wgpu_device().poll(Maintain::Wait);

    // Write the data from buffer slices back to T
    data.bypass_change_detection()
        .map_storage_mappings(&storage_buffer_slices);

    let image_data = render_compute_passes
        .images
        .iter() // Use .iter() to avoid moving/consuming `images`
        .enumerate()
        .map(|(index, (handle, dim))| {
            // Clone handle and dim if necessary; assume slice can be copied or cloned as needed
            let padded_data = &image_buffer_slices[index].get_mapped_range();

            // coverted form padded buffer,
            // TODO was reusing image.data, but dont have access to it here
            let mut image_data = Vec::new();
            for row in 0..dim.height {
                let start = row * dim.padded_bytes_per_row;
                let end = start + dim.unpadded_bytes_per_row;
                image_data.extend_from_slice(&padded_data[start..end]);
            }

            (handle.clone_weak(), image_data)
        })
        .collect::<Vec<_>>();

    if let Err(error) = sender.try_send(ComputeMessage::<T> {
        data: if storage_buffer_slices.len() == 0 {
            debug!("no data to send");
            None
        } else {
            debug!("no data to send");
            Some(data.clone())
        },
        images: image_data,
    }) {
        match error {
            crossbeam_channel::TrySendError::Full(_) => todo!(),
            crossbeam_channel::TrySendError::Disconnected(_) => todo!(),
            // bevy_time::TrySendError::Full(_) => {
            //     panic!("The TimeSender channel should always be empty during render. You might need to add the bevy::core::time_system to your app.",);
            // }
            // bevy_time::TrySendError::Disconnected(_) => {
            //     // ignore disconnected errors, the main world probably just got dropped during shutdown
            // }
        }
    }

    commands.remove_resource::<RenderComputePasses<T>>();

    // TODO: havent noticed any issues without explicitly removing these
    // need to check this
    // drop(storage_buffer_slices);
    // drop(image_buffer_slices);

    //         storage_buffers.iter().for_each(|(_, buffer)| {
    //             buffer.unmap();
    //         });
    //         stage_image.iter().for_each(|(_, buffer, _)| {
    //             buffer.unmap();
    //         });

    //         // notify complete
    //         complete_events.send(ComputeComplete::<T>::default());
}


