use std::fmt::Debug;

use bevy::{
    input::mouse::MouseWheel, prelude::*, render::{
        extract_resource::ExtractResource,
        render_graph::{RenderGraph, RenderLabel},
        render_resource::AsBindGroup,
    }
};
use bevy_inspector_egui::{
    bevy_egui::EguiContexts, inspector_options::{std_options::NumberDisplay, ReflectInspectorOptions}, InspectorOptions
};
use bevy_sly_compute::prelude::*;
use crate::{common_helper::cursor::CursorEvent, DISPATCH_SIZE};

use super::{height_gen::HeightGenLabel, mesh_config::TerrainMeshConfig};

// Our brush to paint the terrain
// There is a flicker with current setup, see https://github.com/slyedoc/bevy_sly_compute/issues/2
#[derive(Reflect, AsBindGroup, ExtractResource, Resource, Debug, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct HeightBrush {

    // For Fun: In this example we are going write directly to the texture
    // you can remove the "stating" part and see brush still paints to the GPU texture
    // but our terrain doesnt see the changes and our Egui UI will not be updated
    #[reflect(ignore)]
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite, staging)]
    pub image: Handle<Image>,

    /// The radius of the brush in uv
    #[uniform(1)]
    #[inspector(min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    pub radius: f32,

    /// The brush pos in uv, will be set on cursor event
    #[reflect(ignore)]
    #[uniform(2)]
    pub position: Vec2,

    /// how much to add to the height, can be negative
    #[uniform(3)]
    pub strength: f32,
}

impl Default for HeightBrush {
    fn default() -> Self {
        Self {
            image: Handle::default(),
            radius: 0.1,
            position: Vec2::new(0.5, 0.5),
            strength: 0.1,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct HeightBrushLabel;

impl ComputeShader for HeightBrush {
    fn shader() -> ShaderRef {
        "terrain/brush.wgsl".into()
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(HeightBrushLabel, ComputeNode::<HeightBrush>::default());
        // since we want height gen to run first
        // TODO: keeping these straight is a bit of a pain and error prone  
        render_graph.add_node_edge(HeightBrushLabel, HeightGenLabel);
    }
}

/// Draw our brush, and trigger the compute event on mouse click
/// Using T  here as a filter to only respond to some entities
pub fn brush_active<T: Component>(
    mut hit_position: EventReader<CursorEvent>,
    mut brush: ResMut<HeightBrush>,
    mut compute_event: EventWriter<ComputeEvent<HeightBrush>>,
    terrain: Res<TerrainMeshConfig>,
    button_input: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
    filter_query: Query<Entity, With<T>>,    
    mut contexts: EguiContexts,
) {    
    // Dont respond to cursor events if we are over an egui area
    // Note: had issues where this only worked like half the time
    let ctx = contexts.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        return;
    }

    // if we have a hit position
    if let Some(event) = hit_position.read().last() {

        // only respond to cursor events on enties with T
        let Ok(_e) = filter_query.get(event.hit.entity) else {
            return;
        };

        // convert brush radius to world space
        let radius = brush.radius * terrain.world_size as f32;

        // update the brush position using hit position 
        // convert to uv space of the 2d texture
        brush.position = Vec2::new(
            (event.pos.x + terrain.world_size as f32 * 0.5)
                / terrain.world_size as f32,
            1.0 - (event.pos.z + terrain.world_size as f32 * 0.5)
                / terrain.world_size as f32,
        );
                    
        if button_input.pressed(MouseButton::Left) {
            gizmos.sphere(event.pos, Quat::IDENTITY, radius, Color::RED);  
            compute_event.send(ComputeEvent::<HeightBrush>::new(DISPATCH_SIZE));
        } else {
            gizmos.sphere(event.pos, Quat::IDENTITY, radius, Color::LIME_GREEN);
        }
    }
}

const STRENGT_TOGGLES: [KeyCode; 2] = [KeyCode::ControlLeft, KeyCode::ControlRight];
pub fn brush_stregnth_toggle(
    mut bursh_settings: ResMut<HeightBrush>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.any_just_pressed(STRENGT_TOGGLES) {
        bursh_settings.strength *= -1.0;
    }
}

const BRUSH_MIN: f32 = 0.02;
const BRUSH_MAX: f32 = 1.0;
const BRUSH_STEP: f32 = 0.01;

pub fn brush_resize(
    mut bursh: ResMut<HeightBrush>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
) {
    for event in mouse_wheel_events.read() {
        if event.y > 0.0 {
            let radius = (bursh.radius + BRUSH_STEP).clamp(BRUSH_MIN, BRUSH_MAX);
            bursh.bypass_change_detection().radius = radius;
        } else {
            let radius = (bursh.radius - BRUSH_STEP).clamp(BRUSH_MIN, BRUSH_MAX);
            bursh.bypass_change_detection().radius = radius;
        }
    }
}
