// ai/agent.rs  (Bevy 0.13 · Rapier 0.25 · NEAT)
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::voxel::grid::{VoxelGrid, GRID_SIZE};
use crate::physics::car_controller::CarInput;
use crate::weapons::{FireWeaponEvent, MountedWeapons};
use super::neat::{OBS_DIM, ACTION_DIM};
use super::neat_core::FeedforwardNet;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum DriveMode {
    Auto,
    Semi,
    Manual,
}

#[derive(Component)]
pub struct CarAgent {
    pub agent_id:           usize,
    pub accumulated_reward: f32,
    pub spawn_pos:          Vec3,
    pub last_position:      Vec3,
    pub weapons_disabled:   bool,
    pub weapon_disable_t:   f32,
    pub immobile_timer:     f32,
    pub out_of_bounds_timer:f32,
    pub anchor_pos:         Vec3,
    pub anchor_timer:       f32,
    pub current_action:     [f32; ACTION_DIM],
    pub drive_mode:         DriveMode,
    pub mode_lock_timer:    f32,
    pub net:                FeedforwardNet,
}

impl CarAgent {
    pub fn new(id: usize, net: FeedforwardNet, spawn_pos: Vec3) -> Self {
        Self {
            agent_id:           id,
            accumulated_reward: 0.,
            spawn_pos,
            last_position:      spawn_pos,
            weapons_disabled:   false,
            weapon_disable_t:   0.,
            immobile_timer:     0.,
            out_of_bounds_timer:0.,
            anchor_pos:         spawn_pos,
            anchor_timer:       0.,
            current_action:     [0.; ACTION_DIM],
            drive_mode:         DriveMode::Semi,
            mode_lock_timer:    0.,
            net,
        }
    }
    pub fn is_alive(&self, grid: &VoxelGrid) -> bool { grid.block_count > 0 }
}

const RAY_COUNT:    usize = 24;
const RAY_MAX_DIST: f32   = 40.0; // Hard limit

pub fn build_observation(
    agent_ent:   Entity,
    agent_tf:    &GlobalTransform,
    vel:         &Velocity,
    grid:        &VoxelGrid,
    rapier_ctx:  &RapierContext,
    other_grids: &[(Entity, GlobalTransform)],
    weapons_cd:  &[f32; 3],
    is_grounded: bool,
    agent_id:    usize,
    nets:        &[GlobalTransform],
    football:    Option<GlobalTransform>,
    discuses:    &[GlobalTransform],
) -> [f32; OBS_DIM] {
    let mut obs = [0.0f32; OBS_DIM];
    let mut idx = 0;

    let mut fwd = agent_tf.forward();
    fwd.y = 0.0; // Parallel to ground plane
    fwd = fwd.normalize_or_zero();
    if fwd == Vec3::ZERO { fwd = Vec3::Z; }
    
    let up  = Vec3::Y; // World up for perfectly horizontal sweeping
    let pos = agent_tf.translation();
    let mat = agent_tf.compute_matrix();
    let inv = mat.inverse();

    let lv = Vec3::new(
        inv.col(0).truncate().dot(vel.linvel),
        inv.col(1).truncate().dot(vel.linvel),
        inv.col(2).truncate().dot(vel.linvel),
    );
    obs[idx] = lv.x/20.; idx+=1;
    obs[idx] = lv.y/20.; idx+=1;
    obs[idx] = lv.z/20.; idx+=1;

    let av = Vec3::new(
        inv.col(0).truncate().dot(vel.angvel),
        inv.col(1).truncate().dot(vel.angvel),
        inv.col(2).truncate().dot(vel.angvel),
    );
    obs[idx] = av.x/5.; idx+=1;
    obs[idx] = av.y/5.; idx+=1;
    obs[idx] = av.z/5.; idx+=1;

    let step = 360.0_f32 / (RAY_COUNT as f32);
    for i in 0..RAY_COUNT {
        // Full 360 degrees around the car
        let angle = (-180. + i as f32 * step).to_radians();
        let dir = Quat::from_axis_angle(up, angle) * fwd;
        
        let car_bit = 1 << agent_id;
        let filter = QueryFilter::default()
            .groups(CollisionGroups::new(Group::from_bits_truncate(car_bit), Group::from_bits_truncate(!car_bit)));
        
        let (nd, ht) = if let Some((_, toi)) = rapier_ctx.cast_ray(pos, dir, RAY_MAX_DIST, true, filter) {
            (toi / RAY_MAX_DIST, 0.5_f32)
        } else { (1.0, 0.0) };
        obs[idx] = nd; idx+=1;
        obs[idx] = ht; idx+=1;
    }

    for y in 0..GRID_SIZE {
        let mut h = 0.; let mut m = 0.;
        for x in 0..GRID_SIZE { for z in 0..GRID_SIZE {
            if let Some(cell) = grid.get(x,y,z) { h += cell.current_health; m += cell.max_health; }
        }}
        obs[idx] = if m > 0. { h/m } else { 0. }; idx+=1;
    }

    let mut best_dist = f32::MAX;
    let mut best_rel  = Vec3::ZERO;
    for (_, gtf) in other_grids {
        let d = gtf.translation().distance(pos);
        if d > 0.1 && d < best_dist { 
            best_dist = d; 
            best_rel = inv.transform_point3(gtf.translation()); 
        }
    }
    obs[idx] = best_rel.x/50.; idx+=1;
    obs[idx] = best_rel.y/20.; idx+=1;
    obs[idx] = best_rel.z/50.; idx+=1;
    obs[idx] = (best_dist/50.).min(1.); idx+=1;
    if nets.len() >= 2 {
        let net0_rel = inv.transform_point3(nets[0].translation());
        let net0_dist = nets[0].translation().distance(pos);
        obs[idx] = net0_rel.x/50.; idx+=1;
        obs[idx] = net0_rel.y/20.; idx+=1;
        obs[idx] = net0_rel.z/50.; idx+=1;
        obs[idx] = (net0_dist/50.).min(1.); idx+=1;

        let net1_rel = inv.transform_point3(nets[1].translation());
        let net1_dist = nets[1].translation().distance(pos);
        obs[idx] = net1_rel.x/50.; idx+=1;
        obs[idx] = net1_rel.y/20.; idx+=1;
        obs[idx] = net1_rel.z/50.; idx+=1;
        obs[idx] = (net1_dist/50.).min(1.); idx+=1;
    } else {
        for _ in 0..8 { obs[idx] = 0.; idx+=1; }
    }

    if let Some(fb) = football {
        let fb_rel = inv.transform_point3(fb.translation());
        let fb_dist = fb.translation().distance(pos);
        obs[idx] = fb_rel.x/50.; idx+=1;
        obs[idx] = fb_rel.y/20.; idx+=1;
        obs[idx] = fb_rel.z/50.; idx+=1;
        obs[idx] = (fb_dist/50.).min(1.); idx+=1;
    } else {
        for _ in 0..4 { obs[idx] = 0.; idx+=1; }
    }

    let mut best_disc_dist = f32::MAX;
    let mut best_disc_rel = Vec3::ZERO;
    for disc in discuses {
        let d = disc.translation().distance(pos);
        if d < best_disc_dist {
            best_disc_dist = d;
            best_disc_rel = inv.transform_point3(disc.translation());
        }
    }
    if best_disc_dist < f32::MAX {
        obs[idx] = best_disc_rel.x/50.; idx+=1;
        obs[idx] = best_disc_rel.y/20.; idx+=1;
        obs[idx] = best_disc_rel.z/50.; idx+=1;
        obs[idx] = (best_disc_dist/50.).min(1.); idx+=1;
    } else {
        for _ in 0..4 { obs[idx] = 0.; idx+=1; }
    }
    obs[idx] = weapons_cd[0]; idx+=1;
    obs[idx] = weapons_cd[1]; idx+=1;
    obs[idx] = weapons_cd[2]; idx+=1;

    obs[idx] = if is_grounded { 1. } else { 0. }; idx+=1;

    obs[idx] = (pos.x/50.).clamp(-1.,1.); idx+=1;
    obs[idx] = (pos.y/20.).clamp(-1.,1.); idx+=1;
    obs[idx] = (pos.z/50.).clamp(-1.,1.);

    obs
}

pub fn apply_action(
    action:      &[f32],
    car_input:   &mut CarInput,
    fire_events: &mut EventWriter<FireWeaponEvent>,
    weapon_ents: &[Entity],
    aim_target:  Vec3,
    disabled:    bool,
) {
    if action.len() < ACTION_DIM { return; }
    
    // Scale sigmoids (0..1) to (-1..1) for steering/throttle
    car_input.throttle = (action[0] * 2.0 - 1.0).clamp(-1., 1.);
    car_input.steer    = (action[1] * 2.0 - 1.0).clamp(-1., 1.);
    
    if disabled { return; }
    
    for (slot, ent) in weapon_ents.iter().take(3).enumerate() {
        if action[2 + slot] > 0.5 { // Threshold sigmoid
            fire_events.send(FireWeaponEvent { weapon_entity: *ent, aim_target });
        }
    }
}

pub fn agent_step_reward_system(
    mut agent_q: Query<(&mut CarAgent, &GlobalTransform, &mut Transform, Option<&mut Velocity>)>,
    time:        Res<Time>,
) {
    let dt = time.delta_seconds();
    for (mut agent, gtf, mut tf, mut vel_opt) in agent_q.iter_mut() {
        let pos   = gtf.translation();
        // Prevent massive distance glitches if physics bugs out
        let moved = pos.distance(agent.last_position).clamp(0.0, 10.0);

        // === REWARD FORMULA ===
        // Damage dealt:   added in damage.rs per hit (+ev.damage)
        // Damage taken:   subtracted below via immobile equivalent (done in damage.rs via victim query)
        
        let mut multiplier = match agent.drive_mode {
            DriveMode::Manual => 10.0,
            DriveMode::Semi => 1.0,
            DriveMode::Auto => 0.25,
        };

        // Distance moved: +0.8 per unit if damage dealt, -0.8 if no damage (only the 50 dummy reward)
        let mut move_reward = moved * 0.8;
        if agent.accumulated_reward <= 50.1 {
            move_reward = moved * -0.8;
        }
        agent.accumulated_reward += move_reward * multiplier;

        // Airborne velocity reward and altitude reward
        if pos.y > 5.0 {
            // Cap altitude reward at y = 150.0 to prevent infinite exploits
            let effective_y = pos.y.min(150.0);
            agent.accumulated_reward += (effective_y / 10.0) * dt * multiplier;

            // Velocity reward
            if let Some(ref vel) = vel_opt {
                let speed = vel.linvel.length();
                agent.accumulated_reward += speed * 0.1 * dt * multiplier;
            }
        }

        // Idle penalty: track net displacement over 1 second
        agent.anchor_timer += dt;
        if agent.anchor_timer >= 1.0 {
            if pos.distance(agent.anchor_pos) < 2.0 {
                // Barely moved from anchor in 1 second
                agent.immobile_timer += 1.0;
                if agent.immobile_timer > 2.0 {
                    agent.accumulated_reward -= 15.0; // penalty per second
                }
            } else {
                // Moved successfully
                agent.immobile_timer = 0.0;
            }
            agent.anchor_pos = pos;
            agent.anchor_timer = 0.0;
        }



        // Out of bounds check
        if pos.x.abs() > 305.0 || pos.z.abs() > 605.0 || pos.y < -10.0 {
            agent.out_of_bounds_timer += dt;
            if agent.out_of_bounds_timer > 5.0 {
                // Teleport back to spawn
                tf.translation = agent.spawn_pos;
                tf.rotation = Quat::IDENTITY;
                if let Some(ref mut vel) = vel_opt {
                    vel.linvel = Vec3::ZERO;
                    vel.angvel = Vec3::ZERO;
                }
                agent.immobile_timer = 0.0;
                agent.out_of_bounds_timer = 0.0;
                agent.last_position = agent.spawn_pos;
                agent.anchor_pos = agent.spawn_pos;
                agent.anchor_timer = 0.0;
                continue;
            }
        } else {
            agent.out_of_bounds_timer = 0.0;
        }

        if agent.weapons_disabled {
            agent.weapon_disable_t -= dt;
            if agent.weapon_disable_t <= 0. {
                agent.weapons_disabled = false;
            }
        }
        if agent.mode_lock_timer > 0. {
            agent.mode_lock_timer -= dt;
        }
        agent.last_position = pos;
    }
}

pub fn ai_step_system(
    mut car_q:       Query<(Entity, &mut CarAgent, &GlobalTransform, &Velocity, &VoxelGrid, &mut CarInput, &MountedWeapons), Without<crate::battle::manager::PlayerControlled>>,
    mut nitro_q:     Query<&mut crate::physics::car_controller::Nitro>,
    other_q:         Query<(Entity, &GlobalTransform), With<crate::battle::manager::CarMarker>>,
    net_q:           Query<&GlobalTransform, With<crate::battle::sports::GoalNet>>,
    football_q:      Query<&GlobalTransform, With<crate::battle::sports::Football>>,
    discus_q:        Query<&GlobalTransform, With<crate::battle::sports::DiscusState>>,
    rapier_ctx:      Res<RapierContext>,
    mut fire_events: EventWriter<FireWeaponEvent>,
) {
    let mut others = Vec::new();
    for (e, tf) in other_q.iter() {
        others.push((e, *tf));
    }
    let nets: Vec<GlobalTransform> = net_q.iter().copied().collect();
    let football = football_q.iter().next().copied();
    let discuses: Vec<GlobalTransform> = discus_q.iter().copied().collect();
    
    // 1. Parallel observation generation and neural net evaluation
    car_q.par_iter_mut().for_each(|(ent, mut agent, gtf, vel, grid, _input, _weapons)| {
        if !agent.is_alive(grid) { return; }
        
        let obs = build_observation(
            ent, gtf, vel, grid, &rapier_ctx, &others,
            &[0., 0., 0.], // cooldowns wired later
            true, // grounded assumed true
            agent.agent_id,
            &nets,
            football,
            &discuses,
        );
        
        let out = agent.net.activate(&obs);
        let mut action = [0.0; ACTION_DIM];
        for i in 0..ACTION_DIM.min(out.len()) {
            action[i] = out[i];
        }
        
        let wants_mode = if action[5] > 0.6 {
            DriveMode::Manual
        } else if action[5] < 0.4 {
            DriveMode::Auto
        } else {
            DriveMode::Semi
        };
        
        if agent.mode_lock_timer <= 0.0 {
            if agent.drive_mode != wants_mode {
                agent.drive_mode = wants_mode;
                agent.mode_lock_timer = 3.0; // 3 second cooldown before switching again
            }
        }
        
        agent.current_action = action;
    });
    
    // 2. Sequential application of outputs (since Events cannot be written to in parallel)
    for (ent, agent, gtf, _vel, grid, mut input, weapons) in car_q.iter_mut() {
        if !agent.is_alive(grid) { continue; }
        
        let aim_target = if agent.drive_mode == DriveMode::Auto {
            // Aimbot: nearest enemy
            let mut nearest_dist = f32::MAX;
            let mut nearest_pos = gtf.translation() + gtf.forward() * 20.0;
            for (other_ent, other_tf) in others.iter() {
                if *other_ent == ent { continue; }
                let dist = gtf.translation().distance(other_tf.translation());
                if dist < nearest_dist {
                    nearest_dist = dist;
                    nearest_pos = other_tf.translation();
                }
            }
            nearest_pos
        } else {
            gtf.translation() + gtf.forward() * 20.0
        };
        
        apply_action(&agent.current_action, &mut input, &mut fire_events, &weapons.weapon_entities, aim_target, agent.weapons_disabled);
        
        // Nitro toggle
        if let Ok(mut nitro) = nitro_q.get_mut(ent) {
            nitro.active = agent.current_action.len() >= 7 && agent.current_action[6] > 0.5;
        }
    }
}

