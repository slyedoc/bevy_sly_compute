mod common_helper;
mod terrain_resources;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin}, prelude::*, render::{render_asset::RenderAssetUsages, render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages}}, window::PrimaryWindow
};
use bevy_inspector_egui::{
    bevy_egui::EguiContext, bevy_inspector, egui, quick::StateInspectorPlugin,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_sly_compute::prelude::*;
use bevy_xpbd_3d::prelude::*;

// Our resources and plugins
use common_helper::{
    cursor::CursorPlugin, cycle_app_state, toggle_wireframe, wake_all_sleeping_bodies, AppState
};
use terrain_resources::{
    brush::{self, HeightBrush},

    height_gen::HeightGen,
    mesh_config::TerrainMeshConfig,
};

use crate::common_helper::MainCamera;

// Sizes of the textures we will be using
const TEXTURE_SIZE: usize = 1024;

// Size of workgroup, shader should match,
const WORKGROUP_SIZE: u32 = 8; // @workgroup_size(8, 8, 1)
                               // Note: I have done almost no testing on workgroup size and is an entire topic on its own

// In this example our dispatch size let us cover the entire texture,
// it completely configurable on per pass per dispatch basis
const DISPATCH_SIZE: UVec3 = UVec3 {
    x: TEXTURE_SIZE as u32 / WORKGROUP_SIZE,
    y: TEXTURE_SIZE as u32 / WORKGROUP_SIZE,
    z: 1,
};

// Marker for our ground
#[derive(Component)]
pub struct Ground;


fn main() {
    App::new()
        .init_state::<AppState>()
        .add_plugins((
            // External plugins
            DefaultPlugins,
            PhysicsPlugins::default(), // Physics
            PhysicsDebugPlugin::default(),
            WireframePlugin,                             // Debuging wireframe
            PanOrbitCameraPlugin,                        // Camera control
            StateInspectorPlugin::<AppState>::default(), // Ui for app state
            
            // Our plugins
            CursorPlugin, // will send events with entity and position on mouse position

            // Our compute plugins, resources with compute shaders
            ComputePlugin::<HeightGen>::default(), // generates random terrain
            ComputePlugin::<HeightBrush>::default(), // our brush
        ))        
        // some settings for our terrain generation from image
        .init_resource::<TerrainMeshConfig>()
        // Create our image, use it to create our resources
        // and setup camera and lighting
        .add_systems(Startup, setup)
        .add_systems(
            Update,            
            // Height Gen - trigger compute shader on change or asset reload
            // Note: this will overwrite current image, if you wanted to persist changes
            // the brush made, you would need 2 more images, one for the brush and 
            // one where you merge them together
            height_compute.run_if(
                resource_changed::<HeightGen>
                    .or_else(on_event::<ComputeShaderModified<HeightGen>>()),
            ),
        )
        .add_systems(
            Update,
            (
                // Most import part of the brush, will draw brush position
                // and trigger compute shader on click
                brush::brush_active::<Ground>,
                // control the brush settings
                brush::brush_resize,
                brush::brush_stregnth_toggle,
            )
                .run_if(in_state(AppState::Brush)),
        )
        .add_systems(
            Update,
            (process_terrain, wake_all_sleeping_bodies).chain().run_if(
                resource_changed::<TerrainMeshConfig>
                    .or_else(image_updated),
            ),
        )
        // UI and debug
        .insert_resource(WireframeConfig {
            default_color: Color::WHITE,
            ..default()
        })
        .add_systems(
            Update,
            (terrain_inspector_ui, toggle_wireframe, cycle_app_state),
        )
        .register_type::<HeightGen>()
        .register_type::<TerrainMeshConfig>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    terrain_config: Res<TerrainMeshConfig>,
    mut config_store: ResMut<GizmoConfigStore>,
    mut compute_terrain: EventWriter<ComputeEvent<HeightGen>>
) {

    // Create a new image
    let mut image = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        // Currently only support binding as Texture Storage
        TextureFormat::Rgba8Unorm, 
        // Since we will be copying this image back and using it        
        RenderAssetUsages::all(),
    );

    // we will be using this image as source and destination for copy, 
    // and using it as a storage texture and texture binding since we 
    // will use it for StandardMaterial
    image.texture_descriptor.usage = TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;

    // Create our inital mesh and colider from the image
    let mesh = terrain_config.generate_mesh(&image);
    let collider = terrain_config.generate_collider(&image);

    let image_handle = images.add(image);
    
    // IMPORTANT: Setting up both resources to use the same image
    commands.insert_resource(HeightGen {
        image: image_handle.clone(),
        ..default()
    });
    // we also want it to run once at startup
    compute_terrain.send(ComputeEvent::<HeightGen>::new(DISPATCH_SIZE));

    commands.insert_resource(HeightBrush {
        image: image_handle.clone(),
        ..default()
    });

    // Create our Terrain
    commands.spawn((
        PbrBundle {            
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial {
                // will use the same image for the texture for now
                base_color_texture: Some(image_handle.clone()),
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        },
        RigidBody::Static,        
        collider,
        Ground,
    ));

    // setup camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 20.0, 20.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera {
            enabled: false,
            ..Default::default()
        },
        MainCamera,
    ));

    // Setup lighting
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..Default::default()
    });


    info!("Press SPACE to toggle brush and camera controls");
    info!("Press F1 to toggle debug wireframe");

    // turn off physics debug
    let (config, _) = config_store.config_mut::<PhysicsGizmos>();
    config.enabled = false;
}

fn process_terrain(
    terrain: Res<HeightGen>,
    terrain_config: Res<TerrainMeshConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    mut query: Query<(&mut Handle<Mesh>, &mut Collider), With<Ground>>,
) {
    // find existing ground
    let Ok((mut mesh_handle, mut collider)) = query.get_single_mut() else {
        warn!("update terrain failed, no ground found");
        return;
    };

    // get updated image from assets
    let image = images.get(&terrain.image).unwrap();

    // generate new mesh and colider from image
    *mesh_handle = meshes.add(terrain_config.generate_mesh(image));
    *collider = terrain_config.generate_collider(image);
}



// Check if the image has been updated
fn image_updated(
    terrain_image: Res<HeightGen>, // could have got the reference from brush or 
                                   // any other resource that uses the same image
    mut asset_events: EventReader<AssetEvent<Image>>
) -> bool {
    asset_events.read().any(|e| match e {
        AssetEvent::Modified { id } => id == &terrain_image.image.id(),
        _ => false,
    })
}

// dispatch compute pass
fn height_compute(mut compute_terrain: EventWriter<ComputeEvent<HeightGen>>) {
    compute_terrain.send(ComputeEvent::<HeightGen>::new(DISPATCH_SIZE));
}

// UI to see everything
fn terrain_inspector_ui(world: &mut World) {
    let egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world);

    let Ok(egui_context) = egui_context else {
        return;
    };
    let mut egui_context = egui_context.clone();

    let mut id = 1337; // TODO: find better way to get unique id
    egui::Window::new("Terrain")
        .default_size((200., 500.))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.heading("Height Gen");
                ui.push_id(id, |ui| {
                    bevy_inspector::ui_for_resource::<HeightGen>(world, ui)
                });
                id = id + 1;

                if ui.add(egui::widgets::Button::new("Reset Image")).clicked() {
                    // we could acces the image and reset data, then trigger HeightGen
                    // but since HeightGen clears everything, we can just set it to changed
                    world.get_resource_mut::<HeightGen>().unwrap()
                        .set_changed();
                }

                ui.heading("Mesh Config");
                ui.push_id(id, |ui| {
                    bevy_inspector::ui_for_resource::<TerrainMeshConfig>(world, ui)
                });
                id = id + 1;

                ui.heading("Brush");
                ui.push_id(id, |ui| {
                    bevy_inspector::ui_for_resource::<HeightBrush>(world, ui)
                });
                id = id + 1;
                ui.label("MouseScroll to resize brush");
                ui.label("Control to flip strength");

                ui.allocate_space(ui.available_size());
            });
        });
}

