// ui/builder.rs
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::battle::manager::{EpisodeState, PlayerControlled, CarMarker};
use crate::voxel::grid::{VoxelGrid, GRID_SIZE};
use crate::voxel::block::BlockType;
use crate::voxel::builder::spawn_block_visuals;
use crate::physics::car_controller::spawn_wheels;
use crate::weapons::MountedWeapons;

#[derive(Resource)]
pub struct BuilderState {
    pub selected_block: BlockType,
    pub grid_entity: Option<Entity>,
}

impl Default for BuilderState {
    fn default() -> Self {
        Self { selected_block: BlockType::Steel, grid_entity: None }
    }
}

pub fn builder_setup(mut commands: Commands) {
    commands.insert_resource(BuilderState::default());
}

pub fn builder_update(
    mut commands: Commands,
    state: Res<State<EpisodeState>>,
    mut next_state: ResMut<NextState<EpisodeState>>,
    mut builder_state: ResMut<BuilderState>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<crate::camera::freecam::FreeCam>>,
    mut grid_q: Query<&mut VoxelGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *state.get() != EpisodeState::PlayerBuild { return; }

    if builder_state.grid_entity.is_none() || grid_q.get(builder_state.grid_entity.unwrap()).is_err() {
        let ent = commands.spawn((
            VoxelGrid::default(),
            Transform::from_translation(Vec3::new(0., 10., 0.)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        builder_state.grid_entity = Some(ent);
    }
    
    if keys.just_pressed(KeyCode::Digit1) { builder_state.selected_block = BlockType::Steel; }
    if keys.just_pressed(KeyCode::Digit2) { builder_state.selected_block = BlockType::Shield; }
    if keys.just_pressed(KeyCode::Digit3) { builder_state.selected_block = BlockType::WheelBig; }

    let window = window_q.single();
    if let Ok((camera, cam_tf)) = camera_q.get_single() {
        if let Some(cursor) = window.cursor_position() {
            if let Some(_ray) = camera.viewport_to_world(cam_tf, cursor) {
                if let Some(grid_ent) = builder_state.grid_entity {
                    if buttons.just_pressed(MouseButton::Left) {
                        if let Ok(mut grid) = grid_q.get_mut(grid_ent) {
                            if grid.block_count == 0 {
                                grid.place(GRID_SIZE/2, GRID_SIZE/2, GRID_SIZE/2, BlockType::PlasmaCore);
                                // Visuals will be spawned on Enter
                            }
                        }
                    }
                }
            }
        }
    }

    if keys.just_pressed(KeyCode::Enter) {
        if let Some(grid_ent) = builder_state.grid_entity {
            if let Ok(mut grid) = grid_q.get_mut(grid_ent) {
                if grid.block_count == 0 { grid.place(GRID_SIZE/2, GRID_SIZE/2, GRID_SIZE/2, BlockType::PlasmaCore); }

                let spawn_pos = Vec3::new(0.0, 15.0, 0.0);
                let agent_id = 19;
                let car_bit = 1 << agent_id;
                
                let player_car = commands.spawn_empty().id();
                spawn_block_visuals(&mut commands, &mut meshes, &mut materials, &mut grid, player_car);
                
                let _wheels = spawn_wheels(&mut commands, &mut meshes, &mut materials, player_car, &grid, spawn_pos, agent_id);

                commands.entity(player_car).insert((
                    CarMarker,
                    PlayerControlled,
                    crate::ai::agent::CarAgent::new(agent_id, crate::ai::neat_core::FeedforwardNet::new(&crate::ai::neat_core::Genome::new(1,1), 1, 1), spawn_pos),
                    SpatialBundle::from_transform(Transform::from_translation(spawn_pos)),
                    RigidBody::Dynamic,
                    Collider::cuboid(2.0, 2.0, 2.0),
                    CollisionGroups::new(Group::from_bits_truncate(car_bit), Group::from_bits_truncate(!car_bit)),
                    ColliderMassProperties::Mass(4000.0),
                    Velocity::default(),
                    grid.clone(),
                    crate::physics::car_controller::CarInput::default(),
                    crate::physics::car_controller::CarDriveState::default(),
                    ExternalImpulse::default(),
                    Damping { linear_damping: 0.1, angular_damping: 0.5 },
                    MountedWeapons { weapon_entities: vec![] },
                ));

                commands.entity(grid_ent).despawn_recursive();
                builder_state.grid_entity = None;
                
                next_state.set(EpisodeState::Battle);
            }
        }
    }
}
