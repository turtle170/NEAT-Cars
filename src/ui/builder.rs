// ui/builder.rs
use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy_rapier3d::prelude::*;
use crate::battle::manager::{PlayerControlled, CarMarker};
use crate::voxel::grid::{VoxelGrid, GRID_SIZE};
use crate::voxel::block::BlockType;
use crate::voxel::builder::{spawn_block_visuals, WeaponPlacement};
use crate::weapons::WeaponType;
use crate::physics::car_controller::spawn_wheels;
use crate::weapons::MountedWeapons;
use crate::camera::freecam::FreeCamState;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum MaterialCategory { Block, Weapon }

#[derive(Resource)]
pub struct BuilderState {
    pub is_open: bool,
    pub category: MaterialCategory,
    pub selected_block_idx: u8,
    pub selected_weapon_idx: u8,
    pub grid_entity: Option<Entity>,
    pub previous_follow: Option<Entity>,
    pub weapon_placements: Vec<WeaponPlacement>,
}

impl Default for BuilderState {
    fn default() -> Self {
        Self { 
            is_open: false,
            category: MaterialCategory::Block,
            selected_block_idx: 1, // Steel
            selected_weapon_idx: 0, // Rpg
            grid_entity: None,
            previous_follow: None,
            weapon_placements: vec![],
        }
    }
}

#[derive(Component)]
pub struct BuilderUiText;

pub fn builder_setup(mut commands: Commands) {
    commands.insert_resource(BuilderState::default());
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle { font_size: 24.0, color: Color::WHITE, ..default() },
        ).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(10.0),
            ..default()
        }),
        BuilderUiText,
    ));
}

pub fn builder_update(
    mut commands: Commands,
    mut builder_state: ResMut<BuilderState>,
    mut cam_state: ResMut<FreeCamState>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scrolls: EventReader<MouseWheel>,
    window_q: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<crate::camera::freecam::FreeCam>>,
    mut grid_q: Query<&mut VoxelGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text_q: Query<&mut Text, With<BuilderUiText>>,
) {
    if !builder_state.is_open { 
        if let Ok(mut text) = text_q.get_single_mut() { text.sections[0].value = "".to_string(); }
        return; 
    }

    if builder_state.grid_entity.is_none() || grid_q.get(builder_state.grid_entity.unwrap()).is_err() {
        let ent = commands.spawn((
            VoxelGrid::default(),
            Transform::from_translation(Vec3::new(0., 1000., 0.)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        )).id();
        builder_state.grid_entity = Some(ent);
        builder_state.previous_follow = cam_state.follow;
        cam_state.follow = Some(ent);
        builder_state.weapon_placements.clear();
    }
    
    if keys.just_pressed(KeyCode::Tab) {
        builder_state.category = if builder_state.category == MaterialCategory::Block { MaterialCategory::Weapon } else { MaterialCategory::Block };
    }

    for ev in scrolls.read() {
        if ev.y > 0.0 {
            if builder_state.category == MaterialCategory::Block {
                builder_state.selected_block_idx = (builder_state.selected_block_idx + 1) % 28;
            } else {
                builder_state.selected_weapon_idx = (builder_state.selected_weapon_idx + 1) % 16;
            }
        } else if ev.y < 0.0 {
            if builder_state.category == MaterialCategory::Block {
                builder_state.selected_block_idx = if builder_state.selected_block_idx == 0 { 27 } else { builder_state.selected_block_idx - 1 };
            } else {
                builder_state.selected_weapon_idx = if builder_state.selected_weapon_idx == 0 { 15 } else { builder_state.selected_weapon_idx - 1 };
            }
        }
    }

    if let Ok(mut text) = text_q.get_single_mut() {
        if builder_state.category == MaterialCategory::Block {
            text.sections[0].value = format!("BUILD MODE [Tab to switch]\nSelected Block: {:?}", BlockType::from_u8(builder_state.selected_block_idx));
        } else {
            text.sections[0].value = format!("BUILD MODE [Tab to switch]\nSelected Weapon: {:?}", WeaponType::from_u8(builder_state.selected_weapon_idx));
        }
    }

    let window = window_q.single();
    if let Ok((camera, cam_tf)) = camera_q.get_single() {
        if let Some(cursor) = window.cursor_position() {
            if let Some(_ray) = camera.viewport_to_world(cam_tf, cursor) {
                if let Some(grid_ent) = builder_state.grid_entity {
                    if buttons.just_pressed(MouseButton::Left) {
                        if let Ok(mut grid) = grid_q.get_mut(grid_ent) {
                            if grid.block_count == 0 {
                                grid.place(GRID_SIZE/2, GRID_SIZE/2, GRID_SIZE/2, BlockType::PlasmaCore);
                                // A simplistic way to register a weapon placement
                                if builder_state.category == MaterialCategory::Weapon {
                                    let w_idx = builder_state.selected_weapon_idx;
                                    builder_state.weapon_placements.push(WeaponPlacement {
                                        grid_pos: [GRID_SIZE/2, GRID_SIZE/2 + 1, GRID_SIZE/2],
                                        weapon_type: WeaponType::from_u8(w_idx),
                                    });
                                }
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
                let agent_id = 31;
                let car_bit = 1_u32 << agent_id;
                
                let player_car = commands.spawn_empty().id();
                spawn_block_visuals(&mut commands, &mut meshes, &mut materials, &mut grid, player_car);
                
                let mut weapons = MountedWeapons::default();
                for wp in &builder_state.weapon_placements {
                    let w_ent = commands.spawn(crate::weapons::WeaponState::new(wp.weapon_type, wp.grid_pos)).id();
                    weapons.weapon_entities.push(w_ent);
                }

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
                    weapons,
                ));

                commands.entity(grid_ent).despawn_recursive();
                builder_state.grid_entity = None;
                builder_state.is_open = false;
                cam_state.follow = Some(player_car); // Follow the newly spawned player car
            }
        }
    }
}
