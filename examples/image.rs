// Generate an image and save it to a file

use std::{path::Path, vec};
use bevy::{
    core_pipeline::tonemapping::Tonemapping, prelude::*, render::{extract_resource::ExtractResource, render_asset::RenderAssetUsages, render_graph::{RenderGraph, RenderLabel}, render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages}, texture::ImageFormat},
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_sly_compute::prelude::*;
use bevy_inspector_egui::{
    prelude::*, quick::ResourceInspectorPlugin
};

const TEXTURE_SIZE: u32 = 1024;
const WORKGROUP_SIZE: u32 = 8; // should match shader

fn main() {
    App::new()
    .add_plugins((
        DefaultPlugins,
        PanOrbitCameraPlugin, // Camera control      
        ComputePlugin::<Simple>::default(), // our compute plugin                
        ResourceInspectorPlugin::<Simple>::default(), // inspector for Simple
    ))
    .init_resource::<Simple>()
    .register_type::<Simple>()
    //run startup and trigger compute shader on start
    .add_systems(Startup, (setup, trigger_computue).chain()) 

    // run compute when resource changes or compute shader is modified
    .add_systems(Update, trigger_computue
        .run_if(resource_changed::<Simple>
            .or_else(on_event::<ComputeShaderModified<Simple>>())
        )
    )
    // Do something when compute is complete
    .add_systems(Last, compute_complete.run_if(on_event::<ComputeComplete<Simple>>()))    
    .run();
}

// AsBindGroupCompute and Resource
#[derive(Reflect, AsBindGroup, ExtractResource, Resource, Debug, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Simple {
    
    // TODO: min max not working
    #[uniform(0, min = 0.0, max = 1.0, display = NumberDisplay::Slider)] 
    offset_x: f32,

    #[uniform(1, min = 0.0, max = 1.0, display = NumberDisplay::Slider)]
    offset_y: f32,

    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite, staging)]
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
            &[0, 0, 0, 255],  // setting to red
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        );
        image.texture_descriptor.usage = TextureUsages::COPY_SRC            
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;
        let image_handle = images.add(image);

        Self {
            offset_x: -0.5,
            offset_y: -0.5,
            image: image_handle,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct SimpleLabel;

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {        
        "image.wgsl".into()
    }

    fn set_nodes(render_graph: &mut RenderGraph) {
        render_graph.add_node(SimpleLabel, ComputeNode::<Simple>::default());
        render_graph.add_node_edge(SimpleLabel, bevy::render::graph::CameraDriverLabel);
    }
}


// helper to trigger compute passes of the correct size
fn trigger_computue(    
    mut compute: EventWriter<ComputeEvent<Simple>>,
) {
    // Size of the dispatch, here we are computing the entire texture
    let dispatch_size = UVec3 { 
        // dispatch size
        x: TEXTURE_SIZE / WORKGROUP_SIZE,
        y: TEXTURE_SIZE / WORKGROUP_SIZE,
        z: 1,
    };
    // You can define the many passes and entry points if you want
    compute.send(ComputeEvent::<Simple> {
        passes: vec![
            Pass {
                entry: "main", // entry point to the shader
                workgroups: vec![dispatch_size],
            },
        ],
        ..default()
    });
    // There are a few helper functions to make this more concise if you dont need all the options
    //compute.send(ComputeEvent::<Simple>::new(dispatch_size));    
}

// Do something when compute is complete
// here we will save the image to a file
fn compute_complete(
    simple: Res<Simple>,
    images: Res<Assets<Image>>
) {

    let path = Path::new("image.png");
    let format = ImageFormat::Png.as_image_crate_format().unwrap();

    info!("Compute complete, saving {path:?} as {format:?}");
    
    // get copy of the image from assets
    let image = images.get(simple.image.clone())
        .unwrap()
        .clone();

    // save the image to a file
    match image.try_into_dynamic() {
        Ok(dyn_img) => {
            let img = dyn_img.to_rgb8();
            match img.save_with_format(&path, format) {
                Ok(_) => info!("Image saved to {}", path.display()),
                Err(e) => error!("Cannot save image, IO error: {e}"),
            }
        }
        Err(e) => error!("Cannot save image, unknown format: {e}"),
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    simple: Res<Simple>,
    mut ambient_light: ResMut<AmbientLight>,
) {
    // disable the default light
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

    // display Handle<Image> 
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::new(Vec3::Y).mesh().size(20.0, 20.0)),                
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                
                base_color_texture: Some(simple.image.clone()),
                unlit: true,
                ..Default::default()
            }),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        },
    )); 
}

