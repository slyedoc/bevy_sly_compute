mod traits;
use std::mem;

use bevy_inspector_egui::bevy_egui::EguiContexts;
pub use traits::*;

mod plugin;
pub use plugin::*;

mod error;
pub use error::*;

mod events;
pub use events::*;

mod pipeline_cache;
pub use pipeline_cache::*;

use bevy::{
    ecs::system::StaticSystemParam,
    prelude::*,
    render::{
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_resource::*,
        renderer::RenderDevice,
        texture::{
            DefaultImageSampler, FallbackImage, FallbackImageCubemap, FallbackImageFormatMsaaCache,
            FallbackImageZero, ImageSamplerDescriptor,
        },
    },
    utils::HashSet,
};

/// This is a helper that will update egui textures when an image is updated
pub struct ComputeEguiPlugin;

impl Plugin for ComputeEguiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ComputeUpdateEgui>().add_systems(
            Update,
            update_egui_textures.run_if(on_event::<ComputeUpdateEgui>()),
        );
    }
}

fn update_egui_textures(
    mut compute_events: EventReader<ComputeUpdateEgui>,
    mut contexts: EguiContexts,
) {
    for event in compute_events.read() {

        let id = contexts.image_id(&event.handle);
        

        match id {
            Some(id) => {
                .
                contexts.remove_image(&event.handle);
                let id2 = contexts.add_image(event.handle.clone());
                info!("update_egui_textures: {:?} {:?}", id, id2);
            }
            None => {

            }
        }
    }
}

/// Helper module to import most used elements.
pub mod prelude {
    pub use crate::{
        events::{ComputePass, *},
        pipeline_cache::{AppPipelineCache, CachedAppComputePipelineId},
        plugin::*,
        traits::*,
        ComputeAssets, ComputePlugin,
    };
    pub use bevy_sly_compute_macros::AsBindGroupCompute;
    // Since these are always used when using this crate
    pub use bevy::{
        reflect::TypeUuid,
        render::render_resource::{ShaderRef, ShaderType},
    };
}

// IMPORTANT: This is a hack, this duplicates ALL images via duplicate RenderAssets<Images> created in world, doubling all texture usage on gpu
// This is a temporary solution to get things working, the goal is to only extract used images
pub struct ComputePlugin {
    /// The default image sampler to use when [`ImageSampler`] is set to `Default`.
    pub default_sampler: ImageSamplerDescriptor,
}

impl Default for ComputePlugin {
    fn default() -> Self {
        ComputePlugin::default_linear()
    }
}

impl ComputePlugin {
    /// Creates image settings with linear sampling by default.
    pub fn default_linear() -> ComputePlugin {
        ComputePlugin {
            default_sampler: ImageSamplerDescriptor::linear(),
        }
    }

    /// Creates image settings with nearest sampling by default.
    pub fn default_nearest() -> ComputePlugin {
        ComputePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }
    }
}

impl Plugin for ComputePlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<Self>() {
            return;
        }

        // update egui textures
        #[cfg(feature = "egui")]
        app.add_event::<ComputeUpdateEgui>()
            .add_systems(
                Update,
                update_egui_textures.run_if(on_event::<ComputeUpdateEgui>()),
            );

        // recreate render assets in app world
        app.init_resource::<ComputeAssets<Image>>()
            .init_resource::<ComputeExtractedAssets<Image>>()
            .init_resource::<ComputePrepareNextFrameAssets<Image>>()
            //.add_systems(Startup, setup)
            .add_systems(PreUpdate, extract_shaders)
            .add_systems(
                Update,
                (
                    process_pipeline_queue_system,
                    extract_compute_asset::<Image>, // ExtractSchedule
                    prepare_assets::<Image>, // AFTER::register_system( render_app, prepare_assets::<A>.in_set(RenderSet::PrepareAssets),
                )
                    .chain(),
            );
    }

    fn finish(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>().clone();

        let default_sampler = render_device.create_sampler(&self.default_sampler.as_wgpu());

        app
            // recreate default resources from render world
            .insert_resource(DefaultImageSampler(default_sampler))
            .init_resource::<FallbackImage>()
            .init_resource::<FallbackImageZero>()
            .init_resource::<FallbackImageCubemap>()
            .init_resource::<FallbackImageFormatMsaaCache>()
            .insert_resource(AppPipelineCache::new(&render_device));
    }
}

// fn setup(
//     mut commands: Commands,
//     render_device: Res<RenderDevice>,
// ) {
//     commands.insert_resource(AppPipelineCache::new(&render_device))
// }

// All Copies from bevy::render::render_resource

#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct ComputeDefaultImageSampler(pub Sampler);

/// Stores all GPU representations ([`RenderAsset::PreparedAssets`](RenderAsset::PreparedAsset))
/// of [`RenderAssets`](RenderAsset) as long as they exist.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ComputeAssets<A: RenderAsset>(pub RenderAssets<A>);

#[derive(Resource, Default)]
pub struct ComputeExtractedAssets<A: RenderAsset> {
    extracted: Vec<(AssetId<A>, A::ExtractedAsset)>,
    removed: Vec<AssetId<A>>,
}

// This is where our types begin

pub struct StageBindGroup {
    pub storage: Vec<(u32, Buffer)>,
    pub handles: Vec<Handle<Image>>,
}

// TODO: not sure I need this
/// All assets that should be prepared next frame.
#[derive(Resource, Default)]
pub struct ComputePrepareNextFrameAssets<A: RenderAsset> {
    pub assets: Vec<(AssetId<A>, A::ExtractedAsset)>,
}

fn process_pipeline_queue_system(
    mut pipeline_cache: ResMut<AppPipelineCache>,
    //mut compute_assets: Res<ComputeAssets<Image>>
    render_device: Res<RenderDevice>,
) {
    let mut waiting_pipelines = mem::take(&mut pipeline_cache.waiting_pipelines);
    let mut pipelines = mem::take(&mut pipeline_cache.pipelines);

    {
        let mut new_pipelines = pipeline_cache.new_pipelines.lock();
        for new_pipeline in new_pipelines.drain(..) {
            let id = pipelines.len();
            pipelines.push(new_pipeline);
            waiting_pipelines.insert(CachedAppComputePipelineId(id));
        }
    }

    for id in waiting_pipelines {
        let pipeline = &mut pipelines[id.0];
        if matches!(pipeline.state, CachedPipelineState::Ok(_)) {
            continue;
        }

        pipeline.state =
            pipeline_cache.process_compute_pipeline(id, &pipeline.descriptor, &render_device);

        if let CachedPipelineState::Err(err) = &pipeline.state {
            match err {
                PipelineCacheError::ShaderNotLoaded(_)
                | PipelineCacheError::ShaderImportNotYetAvailable => {
                    // retry
                    pipeline_cache.waiting_pipelines.insert(id);
                }
                // shader could not be processed ... retrying won't help
                PipelineCacheError::ProcessShaderError(err) => {
                    let error_detail = err.emit_to_string(&pipeline_cache.shader_cache.composer);
                    error!("failed to process shader:\n{}", error_detail);
                    continue;
                }
                PipelineCacheError::CreateShaderModule(description) => {
                    error!("failed to create shader module: {}", description);
                    continue;
                }
            }
        }
    }

    pipeline_cache.pipelines = pipelines;
}

fn extract_shaders(
    mut pipeline_cache: ResMut<AppPipelineCache>,
    shaders: Res<Assets<Shader>>,
    mut events: EventReader<AssetEvent<Shader>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id: shader_id } | AssetEvent::Modified { id: shader_id } => {
                if let Some(shader) = shaders.get(shader_id.clone()) {
                    pipeline_cache.set_shader(shader_id, shader);
                }
            }
            AssetEvent::Removed { id: shader_id } => pipeline_cache.remove_shader(shader_id),
            AssetEvent::LoadedWithDependencies { id: shader_id } => {
                if let Some(shader) = shaders.get(shader_id.clone()) {
                    pipeline_cache.set_shader(shader_id, shader);
                }
            }
        }
    }
}

fn extract_compute_asset<A: RenderAsset>(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<A>>,
    assets: Res<Assets<A>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.extract_asset()));
        }
    }

    commands.insert_resource(ComputeExtractedAssets {
        extracted: extracted_assets,
        removed,
    });
}

/// This system prepares all assets of the corresponding [`RenderAsset`] type
/// which where extracted this frame for the GPU.
pub fn prepare_assets<R: RenderAsset>(
    mut extracted_assets: ResMut<ComputeExtractedAssets<R>>,
    mut render_assets: ResMut<ComputeAssets<R>>,
    mut prepare_next_frame: ResMut<ComputePrepareNextFrameAssets<R>>,
    param: StaticSystemParam<<R as RenderAsset>::Param>,
) {
    let mut param = param.into_inner();
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, extracted_asset) in queued_assets {
        match R::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.0.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_assets.0.remove(removed);
    }

    for (id, extracted_asset) in std::mem::take(&mut extracted_assets.extracted) {
        match R::prepare_asset(extracted_asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.0.insert(id, prepared_asset);
            }
            Err(PrepareAssetError::RetryNextUpdate(extracted_asset)) => {
                prepare_next_frame.assets.push((id, extracted_asset));
            }
        }
    }
}
