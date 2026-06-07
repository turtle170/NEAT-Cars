// camera/freecam.rs  (Bevy 0.13)

use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use crate::voxel::grid::{VoxelGrid, GRID_SIZE, BLOCK_SIZE};

#[derive(Component)]
pub struct FreeCam;

#[derive(Resource)]
pub struct FreeCamState {
    pub yaw:          f32,
    pub pitch:        f32,
    pub position:     Vec3,
    
    pub distance:     f32,
    pub follow:       Option<Entity>,
    pub follow_index: usize,
}

impl Default for FreeCamState {
    fn default() -> Self {
        Self { 
            yaw: 0., 
            pitch: 0.52, 
            position: Vec3::new(0., 20., 40.), 
            distance: 20., 
            follow: None, 
            follow_index: 0 
        }
    }
}

pub fn freecam_system(
    mut cam_q:    Query<&mut Transform, With<FreeCam>>,
    mut state:    ResMut<FreeCamState>,
    mut motions:  EventReader<MouseMotion>,
    mut scrolls:  EventReader<MouseWheel>,
    buttons:      Res<ButtonInput<MouseButton>>,
    keys:         Res<ButtonInput<KeyCode>>,
    time:         Res<Time>,
    grid_q:       Query<(Entity, &GlobalTransform), With<VoxelGrid>>,
) {
    let dt = time.delta_seconds();

    let orbiting = buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle);
    for ev in motions.read() {
        if orbiting {
            state.yaw   -= ev.delta.x * 0.005; // Standard mouse look
            state.pitch -= ev.delta.y * 0.005;
            state.pitch  = state.pitch.clamp(-1.5, 1.5);
        }
    }
    
    let rot = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.);
    let fwd = rot * -Vec3::Z;
    let right = rot * Vec3::X;

    for ev in scrolls.read() {
        if state.follow.is_some() {
            state.distance *= 1.0 - ev.y * 0.1;
            state.distance  = state.distance.clamp(2.0, 120.0);
        } else {
            state.position += fwd * ev.y * 5.0; // zoom/fly in freecam
        }
    }

    if keys.just_pressed(KeyCode::KeyF) { state.follow = None; }

    if keys.just_pressed(KeyCode::Tab) {
        let grids: Vec<Entity> = grid_q.iter().map(|(e,_)| e).collect();
        if !grids.is_empty() {
            state.follow_index = (state.follow_index + 1) % grids.len();
            state.follow = Some(grids[state.follow_index]);
            state.distance = state.distance.min(25.0);
        }
    }

    // WASD pan (First-person Freecam)
    if state.follow.is_none() {
        let mut speed = 25.0 * dt;
        if keys.pressed(KeyCode::ShiftLeft) { speed *= 3.0; }
        
        let mut move_dir = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) { move_dir += fwd; }
        if keys.pressed(KeyCode::KeyS) { move_dir -= fwd; }
        if keys.pressed(KeyCode::KeyA) { move_dir -= right; }
        if keys.pressed(KeyCode::KeyD) { move_dir += right; }
        if keys.pressed(KeyCode::KeyE) { move_dir += Vec3::Y; }
        if keys.pressed(KeyCode::KeyQ) { move_dir -= Vec3::Y; }
        
        if move_dir.length_squared() > 0.0 {
            state.position += move_dir.normalize() * speed;
        }
    }

    // Apply Camera transforms
    for mut tf in cam_q.iter_mut() {
        if let Some(target) = state.follow {
            if let Some((_, gtf)) = grid_q.iter().find(|(e,_)| *e == target) {
                let pivot = gtf.translation();
                let cam_pos = pivot + rot * Vec3::new(0., 0., state.distance);
                tf.translation = cam_pos;
                tf.look_at(pivot, Vec3::Y);
                state.position = cam_pos;
            } else {
                state.follow = None;
            }
        } else {
            tf.translation = state.position;
            tf.rotation = rot;
        }
    }
}



pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera { hdr: true, ..default() },
            transform: Transform::from_xyz(0., 20., 40.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        bevy::core_pipeline::bloom::BloomSettings::default(),
        FreeCam,
    ));
}
