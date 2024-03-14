// Most basic use case, use gpu to calculate a value and return it to the cpu.
use bevy::{prelude::*, render::{extract_resource::ExtractResource, render_graph::{RenderGraph, RenderLabel}, render_resource::AsBindGroup}, window::close_on_esc};
use bevy_inspector_egui::{prelude::*, quick::ResourceInspectorPlugin};
use bevy_sly_compute::prelude::*;
 
// Just like AsBindGroup
#[derive(Reflect, AsBindGroup, Default, ExtractResource, Resource, Clone, Debug, InspectorOptions)
]#[reflect(Resource, InspectorOptions)]
pub struct Simple {
    #[uniform(0)]
    uni: f32,

    #[storage(1, visibility(all), staging)] // added 'staging' have resource updated after compute
    vec: Vec<f32>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SimpleLabel;

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {
        "basic.wgsl".into() 
    }
    
    // set up render graph to control execution order
    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(SimpleLabel, ComputeNode::<Simple>::default());
        render_graph.add_node_edge(SimpleLabel, bevy::render::graph::CameraDriverLabel);
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ComputePlugin::<Simple>::default(),
            ResourceInspectorPlugin::<Simple>::default(), // inspector for Simple
        ))
        
        .insert_resource(Simple {
            uni: 1.0,
            vec: vec![1.0, 2.0, 3.0, 4.0],
        })
        .register_type::<Simple>()
        .add_systems(Startup, setup)
        .add_systems(Update, (trigger_compute, close_on_esc))
        .add_systems(Last, compute_complete.run_if(on_event::<ComputeComplete<Simple>>()))        
        .run();
}

fn trigger_compute(
    keys: Res<ButtonInput<KeyCode>>,
    mut compute_events: EventWriter<ComputeEvent<Simple>>,
    simple: Res<Simple>,
) {
    if keys.just_pressed(KeyCode::Space) {    
        info!("Triggering compute");            
        compute_events.send(ComputeEvent::<Simple>::new_xyz(simple.vec.len() as u32, 1, 1));
    }
}

fn compute_complete( simple: Res<Simple> ) {    
    dbg!(&simple);
}

fn setup(mut commands: Commands) {

    commands.spawn(Camera2dBundle::default());

    info!("Press SPACE to run the compute shader");
    commands.spawn(Text2dBundle {
        text: Text {
            sections: vec![TextSection {
                value: "Press SPACE to run the compute shader\nCheck console".to_string(),
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




