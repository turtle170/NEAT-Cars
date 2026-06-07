// weapons/systems.rs — weapon_cooldown_system + fire_weapon_system (all 16 weapons)
//
// Bevy 0.18 · bevy_rapier3d 0.34

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::battle::damage::SplashDamageRequest;
use crate::voxel::grid::VoxelGrid;
use crate::ai::agent::CarAgent;
use super::{WeaponState, WeaponType, EmpDisabledTimer};
use super::projectile::ProjectileBundle;
use super::homing::HomingTarget;
use super::mine::MineState;

// ─────────────────────────────────────────────────────────────────────────────
// FireWeaponEvent
// ─────────────────────────────────────────────────────────────────────────────

/// Sent by an AI agent or player input to fire a specific weapon.
#[derive(Event, Debug, Clone)]
pub struct FireWeaponEvent {
    /// The weapon entity (must have WeaponState).
    pub weapon_entity: Entity,
    /// World-space point to aim at.
    pub aim_target:    Vec3,
}

// ─────────────────────────────────────────────────────────────────────────────
// StickyBomb helper components
// ─────────────────────────────────────────────────────────────────────────────

/// Marks a sticky-bomb projectile.
#[derive(Component, Debug)]
pub struct StickyBomb {
    /// Time remaining before detonation (after sticking), or total lifetime.
    pub fuse:        f32,
    pub damage:      f32,
    pub radius:      f32,
    pub attacker:    Entity,
    /// Whether the bomb has already stuck to something.
    pub stuck:       bool,
    /// Entity the bomb is stuck to (used to follow its position).
    pub stuck_target: Option<Entity>,
    /// Local offset on the target entity when stuck.
    pub stuck_offset: Vec3,
}

/// Marks a drill weapon entity; drill damage is dealt via contact events.
#[derive(Component, Debug)]
pub struct DrillWeapon {
    pub attacker:    Entity,
    /// Damage per second.
    pub dps:         f32,
    /// Accumulated contact time (seconds) against a given enemy this frame.
    pub accum:       f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// System: weapon_cooldown_system
// ─────────────────────────────────────────────────────────────────────────────

/// Decrements each WeaponState's cooldown timer every frame.
pub fn weapon_cooldown_system(
    time:     Res<Time>,
    mut q:    Query<&mut WeaponState>,
) {
    let dt = time.delta_seconds();
    for mut ws in q.iter_mut() {
        if ws.cooldown > 0.0 {
            ws.cooldown = (ws.cooldown - dt).max(0.0);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: find nearest VoxelGrid enemy entity (excluding `self_entity`)
// ─────────────────────────────────────────────────────────────────────────────

fn nearest_enemy(
    from:        Vec3,
    self_entity: Entity,
    grid_q:      &Query<(Entity, &GlobalTransform), With<VoxelGrid>>,
) -> Option<(Entity, Vec3, f32)> {
    let mut best: Option<(Entity, Vec3, f32)> = None;
    for (ent, gtf) in grid_q.iter() {
        if ent == self_entity { continue; }
        let pos  = gtf.translation();
        let dist = (pos - from).length();
        if best.map_or(true, |(_, _, d)| dist < d) {
            best = Some((ent, pos, dist));
        }
    }
    best
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: random spread direction around `forward`, half-angle in degrees
// ─────────────────────────────────────────────────────────────────────────────

fn spread_dir(forward: Vec3, half_deg: f32, rng: &mut impl Rng) -> Vec3 {
    let half_rad = half_deg.to_radians();
    let yaw   = rng.gen_range(-half_rad..half_rad);
    let pitch = rng.gen_range(-half_rad..half_rad);
    let up    = if forward.abs().dot(Vec3::Y) < 0.99 { Vec3::Y } else { Vec3::X };
    let right = forward.cross(up).normalize_or_zero();
    let up2   = right.cross(forward).normalize_or_zero();
    (forward + right * yaw.tan() + up2 * pitch.tan()).normalize_or_zero()
}

// ─────────────────────────────────────────────────────────────────────────────
// System: fire_weapon_system
// ─────────────────────────────────────────────────────────────────────────────

pub fn is_owner_or_part(hit_ent: Entity, attacker: Entity, owner_q: &Query<&crate::physics::car_controller::Owner>) -> bool {
    if hit_ent == attacker { return true; }
    if let Ok(owner) = owner_q.get(hit_ent) {
        if owner.0 == attacker { return true; }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub fn fire_weapon_system(
    mut commands:   Commands,
    time:           Res<Time>,
    rapier_context: Res<RapierContext>,
    mut events:     EventReader<FireWeaponEvent>,
    mut ws_q:       Query<(&mut WeaponState, &GlobalTransform, Option<&Parent>)>,
    grid_q:         Query<(Entity, &GlobalTransform), With<VoxelGrid>>,
    car_agent_q:    Query<(Entity, &GlobalTransform, &CarAgent)>,
    owner_q:        Query<&crate::physics::car_controller::Owner>,
    mut splash_req: EventWriter<SplashDamageRequest>,
) {
    let dt  = time.delta_seconds();
    let mut rng = rand::thread_rng();

    for ev in events.read() {
        let Ok((mut ws, weapon_gtf, parent_opt)) = ws_q.get_mut(ev.weapon_entity) else { continue };
        if !ws.is_ready() { continue; }

        // Attacker = parent car entity (fallback to weapon entity itself).
        let attacker = parent_opt.map(|p| p.get()).unwrap_or(ev.weapon_entity);
        let muzzle   = weapon_gtf.translation();
        let aim      = ev.aim_target;
        let forward  = (aim - muzzle).normalize_or_zero();

        let car_bit = if let Ok((_, _, agent)) = car_agent_q.get(attacker) { 1 << agent.agent_id } else { 0 };
        let filter_groups = if car_bit != 0 {
            CollisionGroups::new(Group::from_bits_truncate(car_bit), Group::from_bits_truncate(!car_bit))
        } else {
            CollisionGroups::new(Group::ALL, Group::ALL)
        };
        let filter = QueryFilter::new().groups(filter_groups);

        match ws.weapon_type {
            // ─────────────────────────────────────────────────────────────────
            // 0 · RPG — single high-damage rocket
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Rpg => {
                commands.spawn(ProjectileBundle::new(
                    muzzle,
                    forward * 28.0,
                    80.0,
                    3.0,
                    4.0,
                    attacker,
                    1.0,
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 1 · Minigun — 3 raycasts/frame with 4° half-angle spread
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Minigun => {
                for _ in 0..3 {
                    let dir = spread_dir(forward, 4.0, &mut rng);
                    let ray_dir = Vec3::from(dir).normalize_or_zero();
                    if let Some((hit_entity, ray_hit)) = rapier_context.cast_ray_and_get_normal(
                        muzzle,
                        ray_dir,
                        80.0,
                        true,
                        filter,
                    ) {
                        if !is_owner_or_part(hit_entity, attacker, &owner_q) {
                            splash_req.send(SplashDamageRequest {
                                world_pos:   ray_hit.point,
                                radius:      0.1,
                                base_damage: 8.0,
                                attacker,
                            });
                        }
                    }
                }
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 2 · SelfAimingTurret — auto-aims at nearest enemy, fires projectile
            // ─────────────────────────────────────────────────────────────────
            WeaponType::SelfAimingTurret => {
                if let Some((_, enemy_pos, _)) = nearest_enemy(muzzle, attacker, &grid_q) {
                    let auto_dir = (enemy_pos - muzzle).normalize_or_zero();
                    commands.spawn(ProjectileBundle::new(
                        muzzle,
                        auto_dir * 22.0,
                        20.0,
                        0.2,
                        5.0,
                        attacker,
                        0.2,
                    ));
                    ws.reset_cooldown();
                }
            }

            // ─────────────────────────────────────────────────────────────────
            // 3 · Flamethrower — OverlapSphere + 25° cone check → SplashDamage
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Flamethrower => {
                let cone_cos = (25.0_f32).to_radians().cos();
                rapier_context.intersections_with_shape(
                    muzzle,
                    Quat::IDENTITY,
                    &Collider::ball(6.0),
                    filter,
                    |hit_entity| {
                        // Check cone angle.
                        if let Some((_, gtf)) = grid_q.iter().find(|(e, _)| *e == hit_entity) {
                            let to_target = (gtf.translation() - muzzle).normalize_or_zero();
                            if forward.dot(to_target) >= cone_cos {
                                splash_req.send(SplashDamageRequest {
                                    world_pos:   gtf.translation(),
                                    radius:      0.5,
                                    base_damage: 30.0 * dt,
                                    attacker,
                                });
                            }
                        }
                        true // keep iterating
                    },
                );
                // No reset_cooldown — continuous weapon (0s cooldown)
            }

            // ─────────────────────────────────────────────────────────────────
            // 4 · EMPCannon — mark nearby CarAgent entities with EmpDisabledTimer
            // ─────────────────────────────────────────────────────────────────
            WeaponType::EmpCannon => {
                let emp_radius = 8.0_f32;
                for (car_ent, car_gtf, _agent) in car_agent_q.iter() {
                    if car_ent == attacker { continue; }
                    let dist = (car_gtf.translation() - muzzle).length();
                    if dist <= emp_radius {
                        commands.entity(car_ent)
                            .insert(EmpDisabledTimer(4.0));
                    }
                }
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 5 · LaserBeam — single raycast every frame, damage=40*dt
            // ─────────────────────────────────────────────────────────────────
            WeaponType::LaserBeam => {
                let ray_dir = Vec3::from(forward).normalize_or_zero();
                if let Some((hit_entity, ray_hit)) = rapier_context.cast_ray_and_get_normal(
                    muzzle,
                    ray_dir,
                    200.0,
                    true,
                    filter,
                ) {
                    if !is_owner_or_part(hit_entity, attacker, &owner_q) {
                        splash_req.send(SplashDamageRequest {
                            world_pos:   ray_hit.point,
                            radius:      0.05,
                            base_damage: 40.0 * dt,
                            attacker,
                        });
                    }
                }
                // No reset_cooldown — continuous weapon
            }

            // ─────────────────────────────────────────────────────────────────
            // 6 · GrenadeLauncher — arc projectile with upward component
            // ─────────────────────────────────────────────────────────────────
            WeaponType::GrenadeLauncher => {
                let horiz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                let arc_vel = (horiz * 20.0) + Vec3::new(0.0, 12.0, 0.0);
                commands.spawn(ProjectileBundle::new(
                    muzzle,
                    arc_vel,
                    65.0,
                    2.5,
                    3.5,
                    attacker,
                    1.0,
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 7 · Shotgun — 12 raycasts with 18° half-angle spread
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Shotgun => {
                for _ in 0..12 {
                    let dir = spread_dir(forward, 18.0, &mut rng);
                    let ray_dir = Vec3::from(dir).normalize_or_zero();
                    if let Some((hit_entity, ray_hit)) = rapier_context.cast_ray_and_get_normal(
                        muzzle,
                        ray_dir,
                        20.0,
                        true,
                        filter,
                    ) {
                        if !is_owner_or_part(hit_entity, attacker, &owner_q) {
                            splash_req.send(SplashDamageRequest {
                                world_pos:   ray_hit.point,
                                radius:      0.15,
                                base_damage: 10.0,
                                attacker,
                            });
                        }
                    }
                }
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 8 · SniperRailGun — penetrate up to 4 hits, damage falls off
            // ─────────────────────────────────────────────────────────────────
            WeaponType::SniperRailGun => {
                let ray_dir = Vec3::from(forward).normalize_or_zero();
                let mut hits: Vec<(Entity, RayIntersection)> = Vec::new();

                rapier_context.intersections_with_ray(
                    muzzle,
                    ray_dir,
                    500.0,
                    true,
                    filter,
                    |hit_entity, intersection| {
                        if !is_owner_or_part(hit_entity, attacker, &owner_q) {
                            hits.push((hit_entity, intersection));
                        }
                        true // keep going to penetrate
                    },
                );

                // Sort by distance ascending.
                hits.sort_by(|a, b| a.1.toi.partial_cmp(&b.1.toi).unwrap_or(std::cmp::Ordering::Equal));

                for (hit_idx, (_hit_entity, ray_hit)) in hits.iter().enumerate().take(4) {
                    let falloff = 1.0 - hit_idx as f32 * 0.2;
                    let dmg     = 120.0 * falloff;
                    splash_req.send(SplashDamageRequest {
                        world_pos:   ray_hit.point,
                        radius:      0.05,
                        base_damage: dmg,
                        attacker,
                    });
                }
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 9 · HomingMissile — spawn projectile with HomingTarget component
            // ─────────────────────────────────────────────────────────────────
            WeaponType::HomingMissile => {
                if let Some((target_ent, _, _)) = nearest_enemy(muzzle, attacker, &grid_q) {
                    let bundle = ProjectileBundle::new(
                        muzzle,
                        forward * 10.0,
                        60.0,
                        2.0,
                        8.0,
                        attacker,
                        0.0,
                    );
                    commands.spawn(bundle).insert(HomingTarget { target: target_ent });
                    ws.reset_cooldown();
                }
            }

            // ─────────────────────────────────────────────────────────────────
            // 10 · MineLayer — kinematic sensor mine with 1.5s arming delay
            // ─────────────────────────────────────────────────────────────────
            WeaponType::MineLayer => {
                // Drop the mine slightly behind the car.
                let drop_pos = muzzle - forward * 1.2;
                commands.spawn((
                    Transform::from_translation(drop_pos),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    RigidBody::KinematicPositionBased,
                    Collider::ball(0.35),
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    MineState::new(attacker),
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 11 · TeslaCoil — chain lightning up to 4 nearest enemies (5m chain)
            // ─────────────────────────────────────────────────────────────────
            WeaponType::TeslaCoil => {
                let chain_radius = 5.0_f32;
                let max_chains   = 4usize;

                // Gather all grid entities sorted by distance from muzzle.
                let mut enemies: Vec<(Entity, Vec3)> = grid_q.iter()
                    .filter(|(e, _)| *e != attacker)
                    .map(|(e, gtf)| (e, gtf.translation()))
                    .collect();

                // Build chain: each link must be within chain_radius of the previous.
                let mut prev_pos = muzzle;
                let mut chains   = 0usize;

                while chains < max_chains && !enemies.is_empty() {
                    // Find the nearest enemy to prev_pos within chain_radius.
                    let maybe_idx = enemies
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, pos))| (*pos - prev_pos).length() <= chain_radius)
                        .min_by(|(_, (_, a)), (_, (_, b))| {
                            (*a - prev_pos).length()
                                .partial_cmp(&(*b - prev_pos).length())
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i);

                    let Some(idx) = maybe_idx else { break };
                    let (_, enemy_pos) = enemies.remove(idx);

                    let falloff = 1.0 - chains as f32 * 0.2;
                    splash_req.send(SplashDamageRequest {
                        world_pos:   enemy_pos,
                        radius:      0.5,
                        base_damage: 35.0 * falloff,
                        attacker,
                    });

                    prev_pos = enemy_pos;
                    chains  += 1;
                }

                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 12 · PlasmaCannon — slow, high-damage heavy projectile
            // ─────────────────────────────────────────────────────────────────
            WeaponType::PlasmaCannon => {
                commands.spawn(ProjectileBundle::new(
                    muzzle,
                    forward * 8.0,
                    150.0,
                    4.5,
                    12.0,
                    attacker,
                    0.15,
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 13 · Mortar — 65° high arc, large blast
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Mortar => {
                // 65° launch angle from horizontal.
                let horiz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                let angle = 65.0_f32.to_radians();
                let speed = 30.0_f32;
                let arc_vel = horiz * (speed * angle.cos()) + Vec3::Y * (speed * angle.sin());

                commands.spawn(ProjectileBundle::new(
                    muzzle,
                    arc_vel,
                    90.0,
                    3.5,
                    6.0,
                    attacker,
                    1.0,
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 14 · StickyBomb — sticks on first collision, detonates after 3s
            // ─────────────────────────────────────────────────────────────────
            WeaponType::StickyBomb => {
                commands.spawn((
                    Transform::from_translation(muzzle),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    RigidBody::Dynamic,
                    Velocity::linear(forward * 18.0),
                    Collider::ball(0.12),
                    Sensor,
                    ActiveEvents::COLLISION_EVENTS,
                    GravityScale(1.0),
                    StickyBomb {
                        fuse:         5.0, // total lifetime if it never sticks
                        damage:       70.0,
                        radius:       2.8,
                        attacker,
                        stuck:        false,
                        stuck_target: None,
                        stuck_offset: Vec3::ZERO,
                    },
                ));
                ws.reset_cooldown();
            }

            // ─────────────────────────────────────────────────────────────────
            // 15 · Drill — passive ContactDamage, 60 DPS via collision events
            // ─────────────────────────────────────────────────────────────────
            WeaponType::Drill => {
                // The Drill weapon entity itself must already have a Collider
                // attached by the weapon builder; here we ensure DrillWeapon is
                // present so drill_contact_system can do damage.
                commands.entity(ev.weapon_entity).insert(DrillWeapon {
                    attacker,
                    dps:   60.0,
                    accum: 0.0,
                });
                // No cooldown reset — passive
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: sticky_bomb_system — update stuck position + detonate
// ─────────────────────────────────────────────────────────────────────────────

pub fn sticky_bomb_system(
    mut commands:       Commands,
    time:               Res<Time>,
    mut collision_evts: EventReader<CollisionEvent>,
    mut bomb_q:         Query<(Entity, &mut StickyBomb, &GlobalTransform, &mut Velocity, &mut RigidBody)>,
    target_gtf_q:       Query<&GlobalTransform, Without<StickyBomb>>,
    owner_q:            Query<&crate::physics::car_controller::Owner>,
    mut splash_req:     EventWriter<SplashDamageRequest>,
) {
    let dt = time.delta_seconds();

    // Phase 1: process collision events — stick the bomb on first contact.
    for ev in collision_evts.read() {
        let (e1, e2) = match ev {
            CollisionEvent::Started(a, b, _) => (*a, *b),
            CollisionEvent::Stopped(_, _, _) => continue,
        };

        let (bomb_ent, other_ent) = if bomb_q.contains(e1) { (e1, e2) }
                                    else if bomb_q.contains(e2) { (e2, e1) }
                                    else { continue };

        let Ok((_, mut bomb, bomb_gtf, mut vel, mut rb)) = bomb_q.get_mut(bomb_ent) else { continue };
        if bomb.stuck || is_owner_or_part(other_ent, bomb.attacker, &owner_q) { continue; }

        // Stick: freeze the bomb (kinematic) and record offset.
        if let Ok(target_gtf) = target_gtf_q.get(other_ent) {
            let world_offset = bomb_gtf.translation() - target_gtf.translation();
            bomb.stuck        = true;
            bomb.fuse         = 3.0; // detonation fuse starts now
            bomb.stuck_target = Some(other_ent);
            bomb.stuck_offset = world_offset;
            vel.linvel        = Vec3::ZERO;
            vel.angvel        = Vec3::ZERO;
            *rb               = RigidBody::KinematicPositionBased;
        }
    }

    // Phase 2: tick fuse, follow target, detonate.
    for (bomb_ent, mut bomb, _bomb_gtf, _vel, _rb) in bomb_q.iter_mut() {
        bomb.fuse -= dt;

        if bomb.fuse <= 0.0 {
            // Determine world position.
            let world_pos = if let Some(target_ent) = bomb.stuck_target {
                target_gtf_q.get(target_ent)
                    .map(|gtf| gtf.translation() + bomb.stuck_offset)
                    .unwrap_or_else(|_| Vec3::ZERO)
            } else {
                Vec3::ZERO
            };

            splash_req.send(SplashDamageRequest {
                world_pos,
                radius:      bomb.radius,
                base_damage: bomb.damage,
                attacker:    bomb.attacker,
            });

            commands.entity(bomb_ent).despawn();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: drill_contact_system — 60 DPS continuous contact damage
// ─────────────────────────────────────────────────────────────────────────────

pub fn drill_contact_system(
    time:               Res<Time>,
    mut collision_evts: EventReader<CollisionEvent>,
    drill_q:            Query<&DrillWeapon>,
    voxel_q:            Query<&VoxelGrid>,
    gtf_q:              Query<&GlobalTransform>,
    owner_q:            Query<&crate::physics::car_controller::Owner>,
    mut splash_req:     EventWriter<SplashDamageRequest>,
) {
    let dt = time.delta_seconds();

    for ev in collision_evts.read() {
        let (e1, e2) = match ev {
            // We process both Started (begin damage) and Stopped contacts here;
            // damage is applied on every active-contact frame via the Started
            // event repeated by Rapier while overlapping.
            CollisionEvent::Started(a, b, _) => (*a, *b),
            CollisionEvent::Stopped(_, _, _) => continue,
        };

        let (drill_ent, other_ent) = if drill_q.contains(e1) { (e1, e2) }
                                     else if drill_q.contains(e2) { (e2, e1) }
                                     else { continue };

        let Ok(drill) = drill_q.get(drill_ent) else { continue };
        if is_owner_or_part(other_ent, drill.attacker, &owner_q) { continue; }
        if !voxel_q.contains(other_ent) { continue; }

        let Ok(drill_gtf) = gtf_q.get(drill_ent) else { continue };

        splash_req.send(SplashDamageRequest {
            world_pos:   drill_gtf.translation(),
            radius:      0.5,
            base_damage: drill.dps * dt,
            attacker:    drill.attacker,
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: emp_decay_system — count down EmpDisabledTimer, remove when expired
// ─────────────────────────────────────────────────────────────────────────────

pub fn emp_decay_system(
    mut commands: Commands,
    time:         Res<Time>,
    mut emp_q:    Query<(Entity, &mut EmpDisabledTimer)>,
) {
    let dt = time.delta_seconds();
    for (ent, mut timer) in emp_q.iter_mut() {
        timer.0 -= dt;
        if timer.0 <= 0.0 {
            commands.entity(ent).remove::<EmpDisabledTimer>();
        }
    }
}
