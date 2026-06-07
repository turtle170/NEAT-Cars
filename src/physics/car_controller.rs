// physics/car_controller.rs  (Bevy 0.15 · bevy_rapier3d 0.28)

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct CarInput {
    pub throttle: f32,
    pub steer:    f32,
    pub brake:    f32,
}

#[derive(Component)]
pub struct Nitro {
    pub fuel: f32,
    pub active: bool,
}

impl Default for Nitro {
    fn default() -> Self {
        Self { fuel: 100.0, active: false }
    }
}

#[derive(Component)]
pub struct CarWheels {
    pub wheels: Vec<Entity>,
}

#[derive(Component, Default)]
pub struct CarDriveState {
    pub speed:       f32,
    pub is_grounded: bool,
}

#[derive(Component)]
pub struct WheelMarker { pub is_front: bool }

#[derive(Component)]
pub struct AttachedWheel {
    pub grid_pos: [usize; 3],
}

#[derive(Component)]
pub struct WheelTypeMarker(pub crate::voxel::block::BlockType);

#[derive(Component)]
pub struct Owner(pub Entity);

const MAX_FORCE:     f32 = 500.0;
const MAX_STEER:     f32 = 0.52;
const DOWNFORCE:     f32 = 600.0;
const WHEEL_RADIUS:  f32 = 0.25;
const WHEEL_MASS:    f32 = 15.0;

/// Apply throttle/steer → ExternalForce on the car body
/// (simplified: Rapier's ImpulseJoint motor API drives wheels directly)
pub fn car_drive_system(
    mut car_q: Query<(
        Entity,
        &CarInput,
        &Velocity,
        &GlobalTransform,
        &mut ExternalForce,
        &mut CarDriveState,
        &crate::ai::agent::CarAgent,
        &CarWheels,
        &mut Nitro,
    )>,
    mut wheel_q: Query<&mut ExternalForce, Without<CarInput>>,
    rapier_ctx: Res<RapierContext>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for (car_ent, input, vel, gtf, mut ext, mut state, agent, wheels, mut nitro) in car_q.iter_mut() {
        let forward = gtf.forward();
        let up      = gtf.up();
        let pos     = gtf.translation();

        state.speed = vel.linvel.dot(forward);

        // Ground check
        let filter = QueryFilter::default().exclude_rigid_body(car_ent);
        state.is_grounded = rapier_ctx
            .cast_ray(pos, -Vec3::Y, 0.8, true, filter)
            .is_some();

        // Check Mode (Semi vs Manual)
        let is_manual = agent.current_action.len() >= 6 && agent.current_action[5] > 0.6;
        
        let nitro_mult = if nitro.active && nitro.fuel > 0.0 {
            nitro.fuel -= 20.0 * dt; // 5 seconds of nitro
            2.0
        } else {
            nitro.fuel = (nitro.fuel + 10.0 * dt).min(100.0); // 10 seconds to recharge
            1.0
        };

        if is_manual {
            // MANUAL MODE: Apply individual wheel forces directly based on AI outputs
            for (i, &wheel_ent) in wheels.wheels.iter().enumerate() {
                let out_idx = 7 + i; // Index 6 is nitro
                if out_idx < agent.current_action.len() {
                    let wheel_throttle = (agent.current_action[out_idx] * 2.0 - 1.0).clamp(-1.0, 1.0);
                    let wheel_vert = agent.current_action[out_idx].abs(); 
                    if let Ok(mut w_ext) = wheel_q.get_mut(wheel_ent) {
                        w_ext.force = forward * wheel_throttle * 2_083_333.0 * nitro_mult + up * wheel_vert * 833_333.0 * nitro_mult;
                    }
                }
            }
        } else {
            // SEMI MODE: Apply gigantic forces to chassis
            let throttle = if input.throttle.abs() < 0.1 { 1.0 } else { input.throttle }; 
            ext.force = forward * throttle * 6_510_416.0 * nitro_mult + up * throttle.abs() * 2_604_166.0 * nitro_mult;
            ext.torque = up * input.steer * -6_510_416.0;
        }
    }
}

/// Spawn 4 wheel PBR bodies connected via ImpulseJoint
#[derive(Component)]
pub struct PendingJoint {
    pub parent: Entity,
    pub joint: bevy_rapier3d::dynamics::GenericJoint,
}

pub fn spawn_wheels(
    commands:   &mut Commands,
    meshes:     &mut Assets<Mesh>,
    materials:  &mut Assets<StandardMaterial>,
    car_entity: Entity,
    grid:       &crate::voxel::grid::VoxelGrid,
    pos:        Vec3,
    agent_id:   usize,
) -> CarWheels {
    let wheel_mesh = meshes.add(Sphere::new(WHEEL_RADIUS));
    let car_bit = 1 << agent_id;

    let mut ids = Vec::new();
    for x in 0..crate::voxel::grid::GRID_SIZE {
        for y in 0..crate::voxel::grid::GRID_SIZE {
            for z in 0..crate::voxel::grid::GRID_SIZE {
                if let Some(cell) = grid.get(x,y,z) {
                    if cell.block_type.is_wheel() {
                        let offset = crate::voxel::grid::VoxelGrid::cell_local_pos(x,y,z);
                        let anchor = offset - Vec3::new(0.0, 0.05, 0.0);
                        let is_front = z > crate::voxel::grid::GRID_SIZE / 2;

                        let joint = RevoluteJointBuilder::new(Vec3::X)
                            .local_anchor1(anchor)
                            .local_anchor2(Vec3::ZERO)
                            .build();

                        let stats = crate::voxel::block::BlockStats::get(cell.block_type);

                        let id = commands.spawn((
                            PbrBundle {
                                mesh: wheel_mesh.clone(),
                                material: materials.add(StandardMaterial {
                                    base_color: Color::rgb(stats.color[0], stats.color[1], stats.color[2]),
                                    emissive:   Color::rgb(stats.color[0]*30.0, stats.color[1]*30.0, stats.color[2]*30.0),
                                    metallic: 0.2,
                                    perceptual_roughness: 0.8,
                                    ..default()
                                }),
                                transform: Transform::from_translation(pos + anchor),
                                ..default()
                            },
                            RigidBody::Fixed,
                            Collider::ball(WHEEL_RADIUS),
                            ColliderMassProperties::Mass(stats.mass),
                            PendingJoint { parent: car_entity, joint: joint.into() },
                            WheelMarker { is_front },
                            AttachedWheel { grid_pos: [x, y, z] },
                            WheelTypeMarker(cell.block_type),
                            Owner(car_entity),
                            Velocity::default(),
                            ExternalForce::default(),
                            GravityScale(0.0),
                            CollisionGroups::new(Group::from_bits_truncate(car_bit), Group::from_bits_truncate(!car_bit)),
                            ActiveEvents::COLLISION_EVENTS,
                        )).id();
                        ids.push(id);
                    }
                }
            }
        }
    }

    CarWheels { wheels: ids }
}
