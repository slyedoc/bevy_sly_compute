pub mod cursor;

use bevy::{pbr::wireframe::WireframeConfig, prelude::*};
use bevy_panorbit_camera::PanOrbitCamera;
use bevy_xpbd_3d::{components::{Sleeping, TimeSleeping}, plugins::debug::PhysicsGizmos};

#[derive(Component)]
pub struct MainCamera;

#[derive(States, Default, Reflect, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AppState {
    #[default]
    Brush,
    PanOrbit,
}

pub fn cycle_app_state(
    mut cameras: Query<&mut PanOrbitCamera, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let mut camera = cameras.single_mut();

        let next = match state.get() {
            AppState::Brush => AppState::PanOrbit,
            AppState::PanOrbit => AppState::Brush,            
        };

        next_state.set(next);

        // set pan orbit camera
        camera.enabled = match next {
            AppState::PanOrbit => true,
            _ => false,
        };        
    }
}

pub fn toggle_wireframe(
    keys: Res<ButtonInput<KeyCode>>,
    mut wireframe_config: ResMut<WireframeConfig>,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    if keys.just_pressed(KeyCode::F1) {
        wireframe_config.global = !wireframe_config.global;

        let (config, _) = config_store.config_mut::<PhysicsGizmos>();
        config.enabled = !config.enabled;
    }
}

/// Wake up all sleeping bodies,
/// useful if you have colliders on a static collider and it changes
#[allow(dead_code)]
pub fn wake_all_sleeping_bodies(
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut TimeSleeping), With<Sleeping>>,
) {
    for (entity, mut time_sleeping) in &mut bodies {
        commands.entity(entity).remove::<Sleeping>();
        time_sleeping.0 = 0.0;
    }
}