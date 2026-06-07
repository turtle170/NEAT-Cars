// battle/damage.rs  (Bevy 0.15)

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::voxel::block::SpecialAbility;
use crate::voxel::grid::VoxelGrid;
use crate::ai::agent::CarAgent;

#[derive(Event, Debug, Clone)]
pub struct DamageEvent {
    pub target_cell_x: u8,
    pub target_cell_y: u8,
    pub target_cell_z: u8,
    pub damage:        f32,
    pub attacker:      Entity,
    pub died:          bool,
    pub cell_entity:   Option<Entity>,
    pub special:       SpecialAbility,
}

#[derive(Event, Debug)]
pub struct SplashDamageRequest {
    pub world_pos:   Vec3,
    pub radius:      f32,
    pub base_damage: f32,
    pub attacker:    Entity,
}

pub fn process_splash_damage(
    mut requests:      EventReader<SplashDamageRequest>,
    mut grid_q:        Query<(Entity, &GlobalTransform, &mut VoxelGrid)>,
    mut damage_events: EventWriter<DamageEvent>,
    mut agent_q:       Query<&mut CarAgent>,
) {
    for req in requests.read() {
        for (grid_ent, gtf, mut grid) in grid_q.iter_mut() {
            if grid_ent == req.attacker { continue; }
            let local_pos = gtf.compute_matrix().inverse().transform_point3(req.world_pos);
            grid.apply_damage_local(local_pos, req.base_damage, req.radius, req.attacker, &mut damage_events);
        }
        // Reward attacker: +damage_dealt
        if let Ok(mut agent) = agent_q.get_mut(req.attacker) {
            agent.accumulated_reward += req.base_damage;  // raw damage dealt
        }
    }
}

pub fn react_to_damage(
    mut events:     EventReader<DamageEvent>,
    mut commands:   Commands,
    mut agent_q:    Query<&mut CarAgent>,
    transform_q:    Query<&GlobalTransform>,
    parent_q:       Query<&Parent>,
    mut splash_req: EventWriter<SplashDamageRequest>,
) {
    for ev in events.read() {
        // Reward attacker: +damage_dealt
        // Penalise victim:  -damage_taken * 5
        if let Ok(mut attacker_agent) = agent_q.get_mut(ev.attacker) {
            attacker_agent.accumulated_reward += ev.damage;          // +dmg dealt
            if ev.died { attacker_agent.accumulated_reward += 200.0; } // +kill bonus
        }
        // Penalise the victim car (parent of the destroyed cell)
        if let Some(cell_ent) = ev.cell_entity {
            if let Ok(parent) = parent_q.get(cell_ent) {
                let victim_ent = parent.get();
                if victim_ent != ev.attacker {
                    if let Ok(mut victim_agent) = agent_q.get_mut(victim_ent) {
                        victim_agent.accumulated_reward -= ev.damage * 5.0; // -dmg_taken * 5
                    }
                }
            }
        }

        if ev.died {
            if let Some(ent) = ev.cell_entity {
                if let Ok(gtf) = transform_q.get(ent) {
                    let world_tf = gtf.compute_transform();
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    commands.entity(ent)
                        .remove_parent()
                        .insert(world_tf)
                        .insert((
                            RigidBody::Dynamic,
                            Collider::cuboid(0.2, 0.2, 0.2),
                            ExternalImpulse {
                                impulse: Vec3::new(
                                    rng.gen_range(-1.0..1.0),
                                    rng.gen_range(0.5..2.0),
                                    rng.gen_range(-1.0..1.0),
                                ) * 1.5,
                                ..default()
                            },
                            Velocity::default(),
                            crate::battle::manager::FragmentMarker { lifetime: 3.0 },
                        ));
                } else {
                    commands.entity(ent).despawn();
                }

                if ev.special == SpecialAbility::ExplodeOnDeath {
                    if let Ok(gtf) = transform_q.get(ent) {
                        splash_req.send(SplashDamageRequest {
                            world_pos:   gtf.translation(),
                            radius:      1.5,
                            base_damage: 80.0,
                            attacker:    ev.attacker,
                        });
                    }
                }
            }
        }
    }
}

pub fn wheel_contact_damage_system(
    mut collision_events: EventReader<CollisionEvent>,
    wheel_q: Query<(&crate::physics::car_controller::WheelTypeMarker, &crate::physics::car_controller::Owner)>,
    grid_q: Query<Entity, With<crate::voxel::grid::VoxelGrid>>,
    transform_q: Query<&GlobalTransform>,
    mut splash_req: EventWriter<SplashDamageRequest>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(ent1, ent2, _) = event {
            // Check if ent1 is a wheel and ent2 is a grid
            if let Ok((wheel_type, owner)) = wheel_q.get(*ent1) {
                if grid_q.get(*ent2).is_ok() && owner.0 != *ent2 {
                    if let Ok(gtf) = transform_q.get(*ent1) {
                        let damage = match wheel_type.0 {
                            crate::voxel::block::BlockType::WheelSpiked => 15.0,
                            crate::voxel::block::BlockType::WheelHeavy  => 20.0,
                            _ => 5.0,
                        };
                        splash_req.send(SplashDamageRequest {
                            world_pos:   gtf.translation(),
                            radius:      1.0,
                            base_damage: damage,
                            attacker:    owner.0,
                        });
                    }
                }
            }
            // Check if ent2 is a wheel and ent1 is a grid
            if let Ok((wheel_type, owner)) = wheel_q.get(*ent2) {
                if grid_q.get(*ent1).is_ok() && owner.0 != *ent1 {
                    if let Ok(gtf) = transform_q.get(*ent2) {
                        let damage = match wheel_type.0 {
                            crate::voxel::block::BlockType::WheelSpiked => 15.0,
                            crate::voxel::block::BlockType::WheelHeavy  => 20.0,
                            _ => 5.0,
                        };
                        splash_req.send(SplashDamageRequest {
                            world_pos:   gtf.translation(),
                            radius:      1.0,
                            base_damage: damage,
                            attacker:    owner.0,
                        });
                    }
                }
            }
        }
    }
}

pub fn ram_damage_system(
    mut collision_events: EventReader<CollisionEvent>,
    car_q: Query<(&Velocity, &GlobalTransform), With<crate::battle::manager::CarMarker>>,
    mut agent_q: Query<&mut CarAgent>,
    mut splash_req: EventWriter<SplashDamageRequest>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(ent1, ent2, _) = event {
            // Check if both entities are main car bodies
            if let (Ok((vel1, gtf1)), Ok((vel2, gtf2))) = (car_q.get(*ent1), car_q.get(*ent2)) {
                let speed1 = vel1.linvel.length();
                let speed2 = vel2.linvel.length();
                
                let speed_diff = (speed1 - speed2).abs();
                if speed_diff < 5.0 { continue; } // ignore low speed bumps
                
                let (faster_ent, slower_ent, faster_pos) = if speed1 > speed2 {
                    (*ent1, *ent2, gtf1.translation())
                } else {
                    (*ent2, *ent1, gtf2.translation())
                };

                let damage = speed_diff * 2.0;

                // Punish slower car and reward faster car
                if let Ok(mut slower_agent) = agent_q.get_mut(slower_ent) {
                    slower_agent.accumulated_reward -= damage;
                }
                if let Ok(mut faster_agent) = agent_q.get_mut(faster_ent) {
                    faster_agent.accumulated_reward += damage;
                }

                // Apply physical damage to the slower car using splash damage
                splash_req.send(SplashDamageRequest {
                    world_pos:   faster_pos, // Center the damage on the faster car hitting the slower one
                    radius:      2.0,
                    base_damage: damage,
                    attacker:    faster_ent,
                });
            }
        }
    }
}

pub fn block_crumple_visuals_system(
    grid_q: Query<&crate::voxel::grid::VoxelGrid>,
    mut transform_q: Query<&mut Transform>,
) {
    for grid in grid_q.iter() {
        for (_x, _y, _z, cell) in grid.iter_cells() {
            if let Some(ent) = cell.entity {
                if let Ok(mut tf) = transform_q.get_mut(ent) {
                    let frac = cell.health_frac();
                    if frac > 0.0 && frac < 1.0 {
                        let scale = 0.3 + 0.7 * frac;
                        tf.scale = Vec3::splat(scale);
                    }
                }
            }
        }
    }
}
