// Example with egui inspector

use bevy::{
    asset::load_internal_asset, pbr::wireframe::{WireframeConfig, WireframePlugin}, prelude::*, render::{extract_resource::ExtractResource, mesh::Indices, render_asset::RenderAssetUsages, render_resource::*, texture::TextureFormatPixelInfo}, window::{close_on_esc, PrimaryWindow}
};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin},
    bevy_inspector, egui,
    inspector_options::std_options::NumberDisplay,
    prelude::*,
    DefaultInspectorConfigPlugin,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_sly_compute::prelude::*;
use bevy_xpbd_3d::{math::Scalar, prelude::*};

const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11880782407192052050);
const TEXTURE_SIZE: u32 = 64; // size of generated texture
const WORKGROUP_SIZE: u32 = 8; // size of workgroup, shader should match


#[derive(Reflect, AsBindGroup, ExtractResource, Resource, Debug, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Terrain {
    #[uniform(0)]
    #[inspector(min = 0.0, max = 10.0, display = NumberDisplay::Slider)]
    scale: f32,
    
    // staging
    #[storage_texture(1, image_format = Rgba8Unorm, access = ReadWrite, staging)]
    image: Handle<Image>,

    // how big the chunk in world units
    #[uniform(2)]
    #[inspector(min = 1, max = 50, display = NumberDisplay::Slider)]
    world_size: u32,
}

impl ComputeShader for Terrain {
    fn shader() -> ShaderRef {
        ShaderRef::Handle(SHADER_HANDLE)
    }

    fn entry_points<'a>() -> Vec<&'a str> {
        vec!["main"]
    }
}

impl FromWorld for Terrain {
    fn from_world(world: &mut World) -> Self {
        // Create a new red image
        let mut image = Image::new_fill(
            Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            // setting to red
            &[255, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        );
        // set the usage
        image.texture_descriptor.usage = TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING;

        // add the image to the world
        let mut images = world.resource_mut::<Assets<Image>>();
        let image_handle = images.add(image);

        Self {
            scale: 1.0,
            image: image_handle,
            world_size: 10,
        }
    }
}


fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        PhysicsPlugins::default(), // Physics
        WireframePlugin, // Debuging wireframe
        PanOrbitCameraPlugin, // Camera control        
        ComputePlugin::<Terrain>::default(), // Our compute Plugin
        TerrainInspectorPlugin, // Custom inspector below
    ))
    .init_resource::<Terrain>()
    .register_type::<Terrain>()
    .insert_resource(WireframeConfig {
        global: true,
        default_color: Color::WHITE,
    })

    // systems
    .add_systems(Startup, setup)
    .add_systems(Update, trigger_computue.run_if(resource_changed::<Terrain>) )
    .add_systems(Update, close_on_esc)
    .add_systems(
        Update,
        toggle_wireframe,
    )
    .add_systems(Last, process_terrain.run_if(on_event::<ComputeComplete<Terrain>>()));

    // loading internal asset, like editing files side by side
    load_internal_asset!(app, SHADER_HANDLE, "terrain.wgsl", Shader::from_wgsl);

    app.run();
}

// helper to trigger compute passes of the correct size
fn trigger_computue(    
    mut compute_terrain: EventWriter<ComputeEvent<Terrain>>,
) {   
    compute_terrain.send(ComputeEvent::<Terrain> {
        passes: vec![
            Pass {
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
struct Ground;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    terrain: Res<Terrain>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 10.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera::default(),
    ));

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..Default::default()
    });

    // setup our terrain using image
    commands.spawn((
        PbrBundle {
            // mesh will be updated by process_terrain
            mesh: meshes.add(terrain.generate_mesh(&images)),
            material: materials.add(StandardMaterial {
                // image will be updated by process_terrain
                base_color_texture: Some(terrain.image.clone()),
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),

            ..Default::default()
        },
        RigidBody::Static,
        Collider::cuboid(20.0, 0.1, 20.0),
        Ground,
    ));
}

fn process_terrain(
    terrain: Res<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,    
    mut query: Query<(&mut Handle<Mesh>, &mut Collider), With<Ground>>,
) {
    let Ok((mut mesh_handle, mut collider)) = query.get_single_mut() else {
        warn!("update terrain failed, no ground found");
        return;
    };

    // update mesh
    *mesh_handle = meshes.add(terrain.generate_mesh(&mut images));

    // update colider
    *collider = terrain.generate_collider(&images);
}

fn toggle_wireframe(
    keys: Res<ButtonInput<KeyCode>>,
    mut wireframe_config: ResMut<WireframeConfig>
) {
    if keys.just_pressed(KeyCode::F1) {
        wireframe_config.global = !wireframe_config.global;
    }    
}




impl Terrain {
    fn generate_mesh(&self, images: &Assets<Image>) -> Mesh {
        let divisions = TEXTURE_SIZE as usize;
        let half_divisions = divisions as f32 / 2.0;

        let image = images.get(&self.image).unwrap();
        let pixel_size = image.texture_descriptor.format.pixel_size();

        let num_vertices = divisions * divisions;
        let num_indices = (divisions - 1) * (divisions - 1) * 6;

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices as usize);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices as usize);
        let mut indices: Vec<u32> = Vec::with_capacity(num_indices as usize);

        for y in 0..divisions {
            for x in 0..divisions {
                // Calculate the position in the image
                let img_x = (x * divisions) / divisions;
                let img_y = (y * divisions) / divisions;
                let index = (img_y * divisions + img_x) * pixel_size;

                if index + 3 < image.data.len() {
                    let pixel_data = &image.data[index..(index + pixel_size)];

                    let pos = [
                        ((x as f32 - half_divisions) / divisions as f32) * self.world_size as f32,
                        pixel_data[0] as f32 / 255.0,
                        ((y as f32 - half_divisions) / divisions as f32) * self.world_size as f32,
                    ];

                    positions.push(pos);

                    // Vertex index in the positions array
                    let vertex_index = y * divisions + x;

                    uvs.push([x as f32 / divisions as f32, y as f32 / divisions as f32]);

                    if x < divisions - 1 && y < divisions - 1 {
                        let a = vertex_index;
                        let b = vertex_index + divisions;
                        let c = vertex_index + divisions + 1;
                        let d = vertex_index + 1;

                        indices.push(a as u32);
                        indices.push(b as u32);
                        indices.push(c as u32);

                        indices.push(c as u32);
                        indices.push(d as u32);
                        indices.push(a as u32);
                    }
                }
            }
        }

        // build our mesh
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        // compute normals
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();

        // Smooth
        // let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
        // for y in 0..size {
        //     for x in 0..size {
        //         let pos: Vec3 = positions[(y * size + x) as usize].into();
        //         if x < size - 1 && y < size - 1 {
        //             let pos_right: Vec3 = positions[(y * size + x + 1) as usize].into();
        //             let pos_up: Vec3 = positions[((y + 1) * size + x) as usize].into();
        //             let tangent1 = pos_right - pos;
        //             let tangent2 = pos_up - pos;
        //             let normal = tangent2.cross(tangent1);
        //             normals.push(normal.normalize().into());
        //         } else {
        //             normals.push(Vec3::Y.into());
        //         }
        //     }
        // }
        // mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        // mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        mesh
    }

    pub fn generate_collider(&self, images: &Assets<Image>) -> Collider {
        let divisions = TEXTURE_SIZE as usize;
        //let half_divisions = divisions as f32 / 2.0;

        let image = images.get(&self.image).unwrap();
        let pixel_size = image.texture_descriptor.format.pixel_size();

        let mut heights: Vec<Vec<Scalar>> = Vec::with_capacity(divisions);
        let scale = Vec3::new(self.world_size as f32, 1.0, self.world_size as f32);
        for y in 0..divisions {
            let mut row: Vec<Scalar> = Vec::with_capacity((divisions) as usize);
            for x in 0..divisions {
                let img_x = (x * divisions) / divisions;
                let img_y = (y * divisions) / divisions;
                let index = (img_y * divisions + img_x) * pixel_size;

                let _pixel_data = &image.data[index..(index + pixel_size)];
                //row.push(pixel_data[0] as f32 / 255.0);
                row.push((y as f32).sin());
            }
            heights.push(row);
        }
        Collider::heightfield(heights, scale)
    }
}
// This is not ideal in a example
// but small window size from ResourceInspectorPlugin was driving me nuts
// also added button to trigger compute
struct TerrainInspectorPlugin;

impl Plugin for TerrainInspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app.add_systems(Update, terrain_inspector_ui);
    }
}

fn terrain_inspector_ui(world: &mut World) {
    let egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world);

    let Ok(egui_context) = egui_context else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Terrain")
        .default_size((200., 500.))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector::ui_for_resource::<Terrain>(world, ui);
                ui.allocate_space(ui.available_size());
            });
        });
}
