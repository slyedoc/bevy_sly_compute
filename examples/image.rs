use std::vec;

use bevy::{
    prelude::*,
    window::{close_on_esc, PrimaryWindow},
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

const WORKGROUP_SIZE: u32 = 8;
const TEXTURE_SIZE: u32 = 256;
const WORKGROUP: UVec3 = UVec3 {
    x: TEXTURE_SIZE / WORKGROUP_SIZE,
    y: TEXTURE_SIZE / WORKGROUP_SIZE,
    z: 1,
};

fn main() {
    App::new()
    .add_plugins((
        DefaultPlugins,
        ComputeWorkerPlugin::<Simple>::default(),        
        SimpleInspectorPlugin,
    ))
    //.add_plugins()
    .init_resource::<Simple>()
    .register_type::<Simple>()
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
    
    #[uniform(1)]
    color: Color,


    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
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
            // setting to red
            &[255, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
        );
        image.texture_descriptor.usage = TextureUsages::COPY_SRC            
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::TEXTURE_BINDING;
        let image_handle = images.add(image);

        Self {
            fill: 0.5,
            color: Color::RED,
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

fn run_compute(
    keys: Res<Input<KeyCode>>,    
    mut compute_events: EventWriter<ComputeEvent<Simple>>,  
) {
    if keys.just_pressed(KeyCode::Space) {
        compute_events.send(ComputeEvent::<Simple>::new(WORKGROUP));
    }
}

// This is not ideal in a example, but small window size was driving me nuts
// Basiclly a copy of ResourceInspectorPlugin::<Simple>::default() with a different window size
struct SimpleInspectorPlugin;

impl Plugin for SimpleInspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app.add_systems(Update, (run_compute, close_on_esc, simple_inspector_ui));
    }
}

fn simple_inspector_ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world) else {
        return;
    };

    let mut egui_context = egui_context.clone();

    egui::Window::new("Simple")
        .default_size((200., 500.))
        .show(egui_context.get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                bevy_inspector::ui_for_resource::<Simple>(world, ui);

                // button to trigger compute
                ui.separator();

                // Button that span the window
                if ui
                    .add_sized(egui::vec2(ui.available_width(), 30.0), egui::Button::new("Run Compute"))
                    .clicked()
                {
                    let mut worker = world.resource_mut::<ComputeWorker<Simple>>();

                }

                ui.allocate_space(ui.available_size());
            });
        });
}
