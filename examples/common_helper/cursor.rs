use bevy::{ pbr::NotShadowCaster, prelude::*, window::PrimaryWindow};
use bevy_xpbd_3d::prelude::*;

use super::MainCamera;

#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct Picked(pub Vec<Entity>);

/// A 3d cursor and ray caster for mouse
pub struct CursorPlugin;

#[derive(Event)]
pub struct CursorEvent {
    pub pos: Vec3,
    pub hit: RayHitData,
}

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Picked>()
            .init_resource::<Picked>()
            .add_event::<CursorEvent>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, (update_caster, cursor_hits).chain());
            
        //.add_systems(PostUpdate, print_hits);
    }
}

#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub struct PickCaster;

#[derive(Component)]
pub struct Cursor;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        RayCaster::new(Vec3::ZERO, Direction3d::X),
        PickCaster,
        Name::new("MousePicker"),
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(0.1).mesh()),
            material: materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 0.0, 0.0, 0.2),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            //visibility: Visibility { is_visible: true },
            ..default()
        },
        NotShadowCaster,
        Cursor,
        Name::new("Cursor"),
    ));
}

fn update_caster(
    mut raycasters: Query<&mut RayCaster, With<PickCaster>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let mut caster = raycasters.single_mut();
    let window = windows.single();
    let (camera, camera_transform) = camera.single();

    if let Some(cursor_world_pos) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
    {
        caster.origin = cursor_world_pos.origin;
        caster.direction = cursor_world_pos.direction;
    }
}

fn cursor_hits(
    query: Query<(&RayCaster, &RayHits), With<PickCaster>>,
    mut cusror_query: Query<(&mut Transform, &mut Visibility), With<Cursor>>,
    mut cursor_ew: EventWriter<CursorEvent>,
) {
    let Ok((ray, hits)) = query.get_single() else {
        return;
    };

    let Ok((mut cursor_trans, mut cursor_vis)) = cusror_query.get_single_mut() else {
        return;
    };

    if let Some(hit) = hits.iter_sorted().next() {
        *cursor_vis = Visibility::Visible;
        cursor_trans.translation = ray.origin + ray.direction * hit.time_of_impact;
        cursor_ew.send(CursorEvent {
            hit: hit.clone(),
            pos: cursor_trans.translation,
        });
    } else {
        *cursor_vis = Visibility::Hidden;
    }
}
