use std::vec;

use bevy::{
    core_pipeline::tonemapping::Tonemapping, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages}, window::close_on_esc
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_sly_compute::prelude::*;
use bevy_inspector_egui::{
    inspector_options::std_options::NumberDisplay, prelude::*, quick::ResourceInspectorPlugin
};

const TEXTURE_SIZE: u32 = 512;
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
    .add_plugins((
        DefaultPlugins,
        PanOrbitCameraPlugin, // Camera control      
        ComputeWorkerPlugin::<Simple>::default(),                 
        ResourceInspectorPlugin::<Simple>::default(),
    ))
    //.add_plugins()
    .init_resource::<Simple>()
    .register_type::<Simple>()
    
    .add_systems(Startup, setup)
    .add_systems(Update, trigger_computue.run_if(resource_changed::<Simple>()) )
    .add_systems(Last, process_terrain.run_if(on_event::<ComputeComplete<Simple>>()))
    .add_systems(Update, close_on_esc)
    .run();
}

// AsBindGroupCompute and Resource
#[derive(Reflect, AsBindGroupCompute, Resource, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Simple {

    #[inspector(min = 0.0, max = 10.0, display = NumberDisplay::Slider)]
    #[uniform(0)]
    fill: f32,
    
    // TODO: min max not working
    #[uniform(1, min = 0.0, max = 1.0, display = NumberDisplay::Slider)] 
    offset_x: f32,

    #[uniform(2, min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    offset_y: f32,

    #[storage_texture(3, image_format = Rgba8Unorm, access = ReadWrite, staging)]
    pub image: Handle<Image>,    
}

impl FromWorld for Simple {
    fn from_world(world: &mut World) -> Self {

        let mut images = world.resource_mut::<Assets<Image>>();

        let mut image = Image::new_fill(
            Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 0, 0, 255],  // setting to red
            TextureFormat::Rgba8Unorm,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_SRC            
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;
        let image_handle = images.add(image);

        Self {
            fill: 0.5,
            offset_x: -0.5,
            offset_y: -0.5,
            image: image_handle,
        }
    }
}

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {        
        "image.wgsl".into()
    }

    fn entry_points<'a>() -> Vec<&'a str> {
        vec!["main"]
    }
}


// helper to trigger compute passes of the correct size
fn trigger_computue(    
    mut compute: EventWriter<ComputeEvent<Simple>>,
) {
    
    compute.send(ComputeEvent::<Simple> {
        passes: vec![
            ComputePass {
                entry: "main",
                workgroups: vec![UVec3 {
                    // dispatch size
                    x: TEXTURE_SIZE / WORKGROUP_SIZE,
                    y: TEXTURE_SIZE / WORKGROUP_SIZE,
                    z: 1,
                }],
            },
        ],
        ..default()
    });
}

#[derive(Component)]
pub struct Target;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    simple: Res<Simple>,
    mut ambient_light: ResMut<AmbientLight>,
) {

    ambient_light.brightness = 0.0;
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 10.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::None,
            ..Default::default()
        },
        
        PanOrbitCamera::default(),
    ));

    // commands.spawn(DirectionalLightBundle {
    //     transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
    //     ..Default::default()
    // });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: 20.0,
                subdivisions: 10,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(simple.image.clone()),
                unlit: true,
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        },
        Target,
    )); 
}

fn process_terrain(
    terrain: Res<Simple>,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut images: ResMut<Assets<Image>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<&Handle<StandardMaterial>, With<Target>>,
) {
    info!("compute complete");
    let material_handle = query.single_mut();
    let Some(mat) = standard_materials.get_mut(material_handle) else {
        warn!("update terrain failed, no material found");
        return;
    };

    // TODO: Shouldn't need to do this, need to some how flag Image as changed
    mat.base_color_texture = Some(terrain.image.clone());
}