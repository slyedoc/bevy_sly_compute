// Many ComputeWorkerPlugin working together
use bevy::{prelude::*, render::{extract_resource::ExtractResource, render_graph::{RenderGraph, RenderLabel}, render_resource::AsBindGroup}, window::close_on_esc};
use bevy_sly_compute::prelude::*;

#[derive(AsBindGroup, ExtractResource, Resource, Clone, Debug)]
pub struct Simple1 {
    #[uniform(0)]
    uni: f32,

    #[storage(1, visibility(all), staging)] // added 'staging' have resource updated after compute
    vec: Vec<f32>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SimpleLabel1;

impl ComputeShader for Simple1 {
    fn shader() -> ShaderRef {
        "basic.wgsl".into()
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(SimpleLabel1, ComputeNode::<Simple1>::default());
        render_graph.add_node_edge(SimpleLabel1, bevy::render::graph::CameraDriverLabel);
    }
}

#[derive(AsBindGroup, ExtractResource, Resource, Clone, Debug)]
pub struct Simple2 {
    #[uniform(0)]
    uni: f32,

    #[storage(1, visibility(all), staging)]
    vec: Vec<f32>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SimpleLabel2;

impl ComputeShader for Simple2 {
    fn shader() -> ShaderRef {
        "basic.wgsl".into() // could be different shader
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(SimpleLabel2, ComputeNode::<Simple1>::default());
        render_graph.add_node_edge(SimpleLabel2, bevy::render::graph::CameraDriverLabel);
    }

}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ComputePlugin::<Simple1>::default(),
            ComputePlugin::<Simple2>::default(),
        ))
        .insert_resource( Simple1 {
            uni: 1.0,
            vec: vec![1.0, 2.0, 3.0, 4.0],
        })
        .insert_resource( Simple2 {
            uni: -1.0,
            vec: vec![1.0, 2.0],
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (trigger_compute, close_on_esc))
        .add_systems(Update, compute_complete1.run_if(on_event::<ComputeComplete<Simple1>>()))
        .add_systems(Update, compute_complete2.run_if(on_event::<ComputeComplete<Simple2>>()))
        .run();
}

fn trigger_compute(
    keys: Res<ButtonInput<KeyCode>>,
    mut compute_1: EventWriter<ComputeEvent<Simple1>>,
    mut compute_2: EventWriter<ComputeEvent<Simple2>>,
    simple1: Res<Simple1>,
    simple2: Res<Simple2>,
) {
    if keys.just_pressed(KeyCode::Space) {                
        compute_1.send(ComputeEvent::<Simple1>::new_xyz(simple1.vec.len() as u32, 1, 1));
    }
    if keys.just_pressed(KeyCode::Enter) {                
        compute_2.send(ComputeEvent::<Simple2>::new_xyz(simple2.vec.len() as u32, 1, 1));
    }
}

fn compute_complete1( simple1: Res<Simple1>) {
    dbg!(&simple1);
}

fn compute_complete2( simple2: Res<Simple2>) {
    dbg!(&simple2);
}

// Setup a simple 2D camera and text
fn setup(mut commands: Commands) {

    commands.spawn(Camera2dBundle::default());

    info!("Press SPACE to run on first compute shader\nPress ENTER to run on second compute shader");
    commands.spawn(Text2dBundle {
        text: Text {
            sections: vec![TextSection {
                value: "Press SPACE to run the compute shader\nPress ENTER to run second\nCheck console".to_string(),
                style: TextStyle {
                    font_size: 40.0,
                    color: Color::WHITE,
                    ..default()
                },
            }],
            ..Default::default()
        },
        ..Default::default()
    });
}




