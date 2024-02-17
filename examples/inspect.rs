// Example with egui inspector

use bevy::{
    asset::load_internal_asset, prelude::*, window::{close_on_esc, PrimaryWindow},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bevy_sly_compute::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin},
    bevy_inspector, egui,
    inspector_options::std_options::NumberDisplay,
    prelude::*,
    DefaultInspectorConfigPlugin,
};

const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11880782407192052050);
const TEXTURE_SIZE: u32 = 256;  // size of generated texture
const WORKGROUP_SIZE: u32 = 8;  // size of workgroup, shader should match
const WORKGROUP: UVec3 = UVec3 { // dispatch size
    x: TEXTURE_SIZE / WORKGROUP_SIZE,
    y: TEXTURE_SIZE / WORKGROUP_SIZE,
    z: 1,
};

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        ComputeWorkerPlugin::<Simple>::default(),
        SimpleInspectorPlugin,
    ))
    .init_resource::<Simple>()
    .register_type::<Simple>()
    .add_systems(Startup, setup)
    .add_systems(Update, close_on_esc);

    // loading internal asset, like editing files side by side
    load_internal_asset!(app, SHADER_HANDLE, "inspect.wgsl", Shader::from_wgsl);

    app.run();
}

fn setup(mut commands: Commands) {

    commands.spawn(Camera2dBundle::default());

}

#[derive(Reflect, AsBindGroupCompute, Resource, Clone, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct Simple {
    #[inspector(min = -1.0, max = 1.0, display = NumberDisplay::Slider)]
    #[uniform(0, read_only, visibility = "compute")]
    add: f32,

    #[storage(1, staging)]
    vec: Vec<f32>,
    

    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
    pub image: Handle<Image>,    
}

impl FromWorld for Simple {
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
            &[1, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
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
            add: 1.0,
            vec: vec![1.0, 2.0, 3.0, 4.0],
            image: image_handle,
        }
    }
}

impl ComputeShader for Simple {
    fn shader() -> ShaderRef {
        ShaderRef::Handle(SHADER_HANDLE)
    }

    fn entry_points<'a>() -> Vec<&'a str> {
        vec!["pre", "main"]
    }
}

// This is not ideal in a example
// but small window size from ResourceInspectorPlugin was driving me nuts 
// also added button to trigger compute
struct SimpleInspectorPlugin;

impl Plugin for SimpleInspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app.add_systems(Update, simple_inspector_ui);
    }
}

fn simple_inspector_ui(world: &mut World) {
    let egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world);

    let Ok(egui_context) = egui_context else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("Simple")
        .default_size((200., 500.))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector::ui_for_resource::<Simple>(world, ui);

                ui.separator();

                // Button that span the window
                let button_size = egui::vec2(ui.available_width(), 30.0); // Set the desired height to 30.0 or any value you prefer
                if ui
                    .add_sized(button_size, egui::Button::new("Run Compute"))
                    .clicked()
                {
                    let count =world.resource::<Simple>().vec.len() as u32;
                    world.send_event(ComputeEvent::<Simple> {
                        // first pass depends on the vec length                        
                        passes: vec![ComputePass {
                            entry:"pre", 
                            workgroups: vec![
                                UVec3::new(count, 1, 1),
                                UVec3::new(1, 1, 1) // running second pass updating only first position 
                            ],                            
                        },
                        // second pass depends on the image size
                        ComputePass {
                            entry:"main", 
                            workgroups: vec![WORKGROUP],                            
                        }
                        ],
                        ..default()
                    });
                    
                }

                ui.allocate_space(ui.available_size());
            });
        });
}