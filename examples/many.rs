// Many ComputeWorkerPlugin working together
use bevy::{prelude::*, window::close_on_esc};
use bevy_sly_compute::prelude::*;

#[derive(AsBindGroupCompute, Resource, Clone, Debug)]
pub struct Simple1 {
    #[uniform(0)]
    uni: f32,

    #[storage(1, staging)] // added 'staging' have resource updated after compute
    vec: Vec<f32>,
}

impl ComputeShader for Simple1 {
    fn shader() -> ShaderRef {
        "basic.wgsl".into()
    }
}

#[derive(AsBindGroupCompute, Resource, Clone, Debug)]
pub struct Simple2 {
    #[uniform(0)]
    uni: f32,

    #[storage(1, staging)]
    vec: Vec<f32>,
}

impl ComputeShader for Simple2 {
    fn shader() -> ShaderRef {
        "basic.wgsl".into() // couold be different shader
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ComputeWorkerPlugin::<Simple1>::default(),
            ComputeWorkerPlugin::<Simple2>::default(),
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
        .add_systems(Update, (run_compute, log_change, close_on_esc))
        .run();
}

fn run_compute(
    keys: Res<Input<KeyCode>>,
    mut compute_1: EventWriter<ComputeEvent<Simple1>>,
    mut compute_2: EventWriter<ComputeEvent<Simple2>>,
    simple1: Res<Simple1>,
    simple2: Res<Simple2>,
) {
    if keys.just_pressed(KeyCode::Space) {                
        compute_1.send(ComputeEvent::<Simple1>::new_xyz(simple1.vec.len() as u32, 1, 1));
    }
    if keys.just_pressed(KeyCode::Return) {                
        compute_2.send(ComputeEvent::<Simple2>::new_xyz(simple2.vec.len() as u32, 1, 1));
    }
}

fn log_change( simple1: Res<Simple1>, simple2: Res<Simple2> ) {
    if simple1.is_changed() {        
        dbg!(&simple1);
    }    
    if simple2.is_changed() {        
        dbg!(&simple2);
    }    
}

// Setup a simple 2D camera and text
fn setup(mut commands: Commands) {

    commands.spawn(Camera2dBundle::default());

    info!("Press SPACE to run on first compute shader\nPress RETURN to run on second compute shader");
    commands.spawn(Text2dBundle {
        text: Text {
            sections: vec![TextSection {
                value: "Press SPACE to run the compute shader\nPress RETURN to run second\nCheck console".to_string(),
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




