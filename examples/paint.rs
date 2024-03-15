mod common_helper;
mod paint_resources;

// Most basic use case, use gpu to calculate a value and return it to the cpu.

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, StateInspectorPlugin};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_sly_compute::prelude::*;
use bevy_xpbd_3d::{
    components::RigidBody,
    plugins::{collision::Collider, debug::PhysicsGizmos, PhysicsDebugPlugin, PhysicsPlugins},
};

use common_helper::{cursor::CursorPlugin, *};
use paint_resources::brush::{self, Brush};

// Size of workgroup, shader should match,
// Note: I have done almost no testing on workgroup size and is an entire topic on its own
const WORKGROUP_SIZE: u32 = 8; // @workgroup_size(8, 8, 1)
                               
fn main() {
    App::new()
        .init_state::<AppState>()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(), // Physics
            PhysicsDebugPlugin::default(),
            WireframePlugin,                             // Debuging wireframe
            PanOrbitCameraPlugin,                        // Camera control
            ResourceInspectorPlugin::<Brush>::default(), // inspector for Simple
            StateInspectorPlugin::<AppState>::default(), // Ui for app state

            // our plugins
            CursorPlugin, // will send events with entity and position on mouse position
            ComputePlugin::<Brush>::default(),
        ))
        .init_resource::<Brush>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // Most import part of the brush, will draw brush position
                // and trigger compute shader on click
                brush::brush_active,
                // control the brush settings
                brush::brush_resize,
            )
                .run_if(in_state(AppState::Brush)),
        )
        // UI and debug
        .insert_resource(WireframeConfig {
            default_color: Color::WHITE,
            ..default()
        })
        .add_systems(Update, (toggle_wireframe, cycle_app_state))
        .register_type::<Brush>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    // setup camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 5.0, -1.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera {
            enabled: false,
            ..Default::default()
        },
        MainCamera,
    ));

    // create grid of targets we can paint on
    let mesh = meshes.add(Plane3d::new(Vec3::Y).mesh().size(1.0, 1.0));

    let spacing = 1.2;
    let half_size = spacing / 2.0;
    let grid = 5;
    for y in 0..grid {
        for x in 0..grid {
            let x = x as f32 * spacing - (half_size * (grid as f32 - 1.0));
            let z = y as f32 * spacing - (half_size * (grid as f32 - 1.0));
            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material: materials.add(StandardMaterial {
                        base_color_texture: Some(images.add(new_image(512))),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    }),
                    transform: Transform::from_translation(Vec3::new(x, 0.0, z)),
                    ..default()
                },
                RigidBody::Static,
                Collider::cuboid(1.0, 0.1, 1.0),
            ));
        }
    }

    // Big paint target off to the right
    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(images.add(new_image(1024))),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),   
            transform: Transform::from_translation(Vec3::new(-10.0, 0.0, 0.0))
            .with_scale(Vec3::new(10.0, 1.0, 10.0)),         
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(1.0, 0.1, 1.0),
    ));


    info!("Press SPACE to toggle brush and camera");
    info!("Press F1 to toggle debug wireframe");

    // turn off physics debug
    let (config, _) = config_store.config_mut::<PhysicsGizmos>();
    config.enabled = false;
}

pub fn new_image(size: u32) -> Image {
    // Create a new image
    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        // Currently only support binding as Texture Storage
        TextureFormat::Rgba8Unorm,
        // Since we will be copying this image back and using it
        RenderAssetUsages::all(),
    );

    // we are not staging these so no need for COPY_DST and COPY_SRC
    // will will use storage texture and texture binding since since 
    // there used for StandardMaterial
    image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING 
        | TextureUsages::TEXTURE_BINDING;

    image
}
