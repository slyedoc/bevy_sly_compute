// Most basic use case
use bevy::{prelude::*, window::close_on_esc};
use bevy_sly_compute::prelude::*;

// Just like AsBindGroup
#[derive(AsBindGroupCompute, Resource, Clone, Debug)]
pub struct Simple {
    #[uniform(0)]
    uni: f32,

    #[storage(1, staging)] // added 'staging' have resource updated after compute
    vec: Vec<f32>,
}

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {
        "basic.wgsl".into() // Asset path to the shader 
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ComputeWorkerPlugin::<Simple>::default(),
        ))
        .insert_resource( Simple {
            uni: 1.0,
            vec: vec![1.0, 2.0, 3.0, 4.0],
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (run_compute, log_change, close_on_esc))
        .run();
}

fn run_compute(
    keys: Res<Input<KeyCode>>,
    mut compute_events: EventWriter<ComputeEvent<Simple>>,
    simple: Res<Simple>,
) {
    if keys.just_pressed(KeyCode::Space) {                
        compute_events.send(ComputeEvent::<Simple>::new_xyz(simple.vec.len() as u32, 1, 1));
    }
}

fn log_change( simple: Res<Simple> ) {
    if simple.is_changed() {        
        dbg!(&simple);
    }    
}

// Setup a simple 2D camera and text
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




