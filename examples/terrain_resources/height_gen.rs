use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::AsBindGroup,
    },
};
use bevy_inspector_egui::inspector_options::std_options::NumberDisplay;
use bevy_inspector_egui::{inspector_options::ReflectInspectorOptions, InspectorOptions};
use bevy_sly_compute::prelude::*;

#[derive(Reflect, AsBindGroup, ExtractResource, Resource, Debug, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct HeightGen {
    // Since we are using this for height data, using R32Float here is pretty nice
    // but for this example I will use Rgba8Unorm to keep it simpler
    #[storage_texture(0, image_format = Rgba8Unorm, access = WriteOnly, staging)]
    pub image: Handle<Image>,

    #[uniform(1)]
    pub offset: Vec2,

    #[uniform(2)]
    #[inspector(min = 0.0, max = 10.0, display = NumberDisplay::Slider)]
    pub scale: f32,
}

impl Default for HeightGen {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: Vec2::ZERO,
            image: Handle::default(),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct HeightGenLabel;

impl ComputeShader for HeightGen {
    fn shader() -> ShaderRef {
        "terrain/height_gen.wgsl".into()
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(HeightGenLabel, ComputeNode::<Self>::default());
        // attach to camera driver since this will be ran first
        render_graph.add_node_edge(HeightGenLabel, bevy::render::graph::CameraDriverLabel);
    }
}
