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
use crate::{common_helper::cursor::CursorEvent, WORKGROUP_SIZE};


// Our brush to paint any thing with standard material
// There is a flicker with current setup, see https://github.com/slyedoc/bevy_sly_compute/issues/2
#[derive(Reflect, AsBindGroup, ExtractResource, Resource, Debug, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Brush {

    // For Fun: In this example we are going write directly to the texture
    // you can remove the "stating" part and see brush still paints to the GPU texture
    // but our terrain doesnt see the changes and our Egui UI will not be updated
    #[reflect(ignore)]
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadWrite)]
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
    pub color: Color,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            image: Handle::default(),
            radius: 0.1,
            position: Vec2::new(0.5, 0.5),
            color: Color::RED,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct HeightBrushLabel;

impl ComputeShader for Brush {
    fn shader() -> ShaderRef {
        "paint/brush.wgsl".into()
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(HeightBrushLabel, ComputeNode::<Brush>::default());
        // since we want height gen to run first
        // TODO: keeping these straight is a bit of a pain and error prone  
        render_graph.add_node_edge(HeightBrushLabel, bevy::render::graph::CameraDriverLabel);
    }
}

/// Draw our brush, and trigger the compute event on mouse click
pub fn brush_active(
    mut hit_position: EventReader<CursorEvent>,
    mut brush: ResMut<Brush>,
    mut compute_event: EventWriter<ComputeEvent<Brush>>,
    button_input: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
    filter_query: Query<(&Handle<StandardMaterial>, &Transform)>, 
    standard_materials: Res<Assets<StandardMaterial>>,   
    images: Res<Assets<Image>>,   
    mut contexts: EguiContexts,
) {
    let ctx = contexts.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
        return;
    }

    // if we have a hit position
    if let Some(event) = hit_position.read().last() {

        // check the entity has a material
        let Ok((mat, trans)) = filter_query.get(event.hit.entity) else {
            return;
        };

        // get the material and set its image if precent
        let material = standard_materials.get(mat).unwrap();
        let handle = match &material.base_color_texture {
            Some(handle) => handle,
            None => return, // dont have a texture to paint
        };
        brush.image = handle.clone();

        // convert cursor position to uv space
        // this only works this easy because all target are 1u
        // and not rotated, not sure how you would get the uv from a mesh
        let target_pos = trans.translation - event.pos;        
        brush.position = target_pos.xz() ;
        brush.position /= trans.scale.xz();  // need this for big one
        brush.position += Vec2::new(0.5, 0.5);
        brush.position.x = 1.0 - brush.position.x; // flip x
        //dbg!(brush.position);

        // clamp to uv space
        brush.position = brush.position.clamp(Vec2::ZERO, Vec2::ONE);
        
        // checky trick to work with our big target
        let world_radius = brush.radius * trans.scale.x;

        if button_input.pressed(MouseButton::Left) {
            gizmos.sphere(event.pos, Quat::IDENTITY, world_radius, Color::RED);  

            // we need image size for dispatch size
            let image = images.get(handle).unwrap();            
            compute_event.send(ComputeEvent::<Brush>::new_xyz(image.width() / WORKGROUP_SIZE, image.height() / WORKGROUP_SIZE, 1));
        } else {
            gizmos.sphere(event.pos, Quat::IDENTITY, world_radius, Color::LIME_GREEN);
        }
    }
}


const BRUSH_MIN: f32 = 0.02;
const BRUSH_MAX: f32 = 1.0;
const BRUSH_STEP: f32 = 0.01;

pub fn brush_resize(
    mut bursh: ResMut<Brush>,
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
