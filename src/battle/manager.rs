// battle/manager.rs  (Bevy 0.13 · NEAT)
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::voxel::grid::VoxelGrid;
use crate::voxel::builder::{apply_genome, random_genome, random_weapons, spawn_block_visuals, CarBuilder};
use crate::physics::car_controller::{CarInput, CarDriveState, spawn_wheels};
use crate::ai::agent::CarAgent;
use crate::ai::neat::{NeatManager, POPULATION_SIZE};
use crate::ai::neat_core::FeedforwardNet;
use crate::weapons::MountedWeapons;
use crate::ai::neat::{OBS_DIM, ACTION_DIM};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum EpisodeState { #[default] Reset, Build, Battle }

#[derive(Resource, Default)]
pub struct EpisodeInfo {
    pub episode:     u32,
    pub phase_timer: f32,
    pub agent_count: usize,
    pub next_spawn_idx: usize,
    pub build_frame_counter: usize,
}

#[derive(Component)]
pub struct FragmentMarker { pub lifetime: f32 }

#[derive(Component)]
pub struct CarMarker;

#[derive(Component)]
pub struct PlayerControlled;

#[derive(Component)]
pub struct HexPlatform {
    pub min_reward: f32,
    pub fitness_boost: f32,
}

const BUILD_TIME: f32 = 8.0;
const BATTLE_TIMEOUT: f32 = 60.0; // 60s per generation
const RESET_PAUSE:    f32 = 2.0;

pub fn episode_state_machine(
    state:     Res<State<EpisodeState>>,
    mut next:  ResMut<NextState<EpisodeState>>,
    mut info:  ResMut<EpisodeInfo>,
    time:      Res<Time>,
    _car_q:     Query<&VoxelGrid, With<CarMarker>>,
) {
    info.phase_timer -= time.delta_seconds();
    match state.get() {
        EpisodeState::Reset  => { if info.phase_timer <= 0. { next.set(EpisodeState::Build);  info.phase_timer = BUILD_TIME;    info.episode += 1; } }
        EpisodeState::Build  => { if info.phase_timer <= 0. { next.set(EpisodeState::Battle); info.phase_timer = BATTLE_TIMEOUT; } }
        EpisodeState::Battle => {
            if (info.next_spawn_idx >= POPULATION_SIZE && info.agent_count <= 1) || info.phase_timer <= 0. { 
                next.set(EpisodeState::Reset); 
                info.phase_timer = RESET_PAUSE; 
            }
        }
    }
}

pub fn on_enter_build(
    mut info: ResMut<EpisodeInfo>,
    mut time: ResMut<Time<Virtual>>,
) {
    info.next_spawn_idx = 0;
    info.build_frame_counter = 0;
    time.set_relative_speed(1.0);
}

pub fn sequential_build_system(
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    neat_mgr:      Res<NeatManager>,
    mut info:      ResMut<EpisodeInfo>,
    state:         Res<State<EpisodeState>>,
) {
    if *state.get() != EpisodeState::Build { return; }
    
    info.build_frame_counter += 1;
    // Spawn 1 car every 15 frames
    if info.build_frame_counter % 15 == 0 && info.next_spawn_idx < POPULATION_SIZE && info.agent_count < 20 {
        spawn_single_car(&mut commands, &mut meshes, &mut materials, &neat_mgr, info.next_spawn_idx);
        info.next_spawn_idx += 1;
        info.agent_count += 1;
    }
}

pub fn spawn_single_car(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    neat_mgr:  &NeatManager,
    agent_id:  usize,
) {
    // Grid layout for designated locations: 5x6
    let col = (agent_id % 5) as f32;
    let row = (agent_id / 5) as f32;
    let pos = Vec3::new(col * 35.0 - 70.0, 15.0, row * 35.0 - 35.0);

    let genome  = random_genome();
    let weapons = random_weapons(3);

    let mut grid = VoxelGrid::default();
    apply_genome(&mut grid, &genome, &weapons);

    let net = FeedforwardNet::new(&neat_mgr.population[agent_id], OBS_DIM, ACTION_DIM);

    let car_ent = commands.spawn_empty().id();
    spawn_block_visuals(commands, meshes, materials, &mut grid, car_ent);

    let wheels = spawn_wheels(commands, meshes, materials, car_ent, &grid, pos, agent_id);

    let car_bit = 1 << agent_id;

    commands.entity(car_ent).insert((
        SpatialBundle::from_transform(Transform::from_translation(pos)),
        RigidBody::Fixed,
        Collider::cuboid(2.0, 2.0, 2.0),
        CollisionGroups::new(Group::from_bits_truncate(car_bit), Group::from_bits_truncate(!car_bit)),
        ColliderMassProperties::Mass(4000.0),
        Velocity::default(),
        ExternalForce::default(),
        Friction { coefficient: 0.8, combine_rule: CoefficientCombineRule::Max },
    )).insert((
        CarInput::default(),
        CarDriveState::default(),
        grid,
        CarBuilder::default(),
        CarAgent::new(agent_id, net, pos),
        crate::physics::car_controller::Nitro::default(),
        MountedWeapons::default(),
        wheels,
        Name::new(format!("Car_{}", agent_id)),
        CarMarker,
        ExternalImpulse::default(),
    )).insert((
        Damping { linear_damping: 0.3, angular_damping: 0.5 },
        GravityScale(0.0), // Frozen initially
    ));
}

pub fn car_lifecycle_system(
    mut commands: Commands,
    mut info: ResMut<EpisodeInfo>,
    mut neat_mgr: ResMut<NeatManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    car_q: Query<(Entity, &VoxelGrid, &CarAgent), With<CarMarker>>,
    state: Res<State<EpisodeState>>,
) {
    if *state.get() == EpisodeState::Reset { return; }

    let mut active = 0;
    
    for (ent, grid, agent) in car_q.iter() {
        if grid.block_count == 0 {
            if agent.agent_id < POPULATION_SIZE {
                neat_mgr.population[agent.agent_id].fitness = agent.accumulated_reward;
            }
            commands.entity(ent).despawn_recursive();
        } else {
            active += 1;
        }
    }

    if *state.get() == EpisodeState::Battle {
        while active < 20 && info.next_spawn_idx < POPULATION_SIZE {
            spawn_single_car(&mut commands, &mut meshes, &mut materials, &neat_mgr, info.next_spawn_idx);
            info.next_spawn_idx += 1;
            active += 1;
        }
    }
    
    info.agent_count = active;
}

pub fn player_control_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut CarInput, With<PlayerControlled>>,
) {
    for mut input in q.iter_mut() {
        let mut throttle = 0.0;
        let mut steer = 0.0;
        
        if keyboard.pressed(KeyCode::KeyW) { throttle += 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { throttle -= 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { steer += 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { steer -= 1.0; }
        
        input.throttle = throttle;
        input.steer = steer;
        // input.brake = 0.0; // Currently unhandled explicitly in WASD
    }
}

pub fn on_enter_reset(
    mut commands: Commands,
    car_q:        Query<(Entity, &CarAgent), With<CarMarker>>,
    frag_q:       Query<Entity, With<FragmentMarker>>,
    wheel_q:      Query<Entity, With<crate::physics::car_controller::WheelMarker>>,
    projectile_q: Query<Entity, With<crate::weapons::projectile::Projectile>>,
    mut neat_mgr: ResMut<NeatManager>,
) {
    // Collect fitness before destroying
    for (ent, agent) in car_q.iter() {
        if agent.agent_id < POPULATION_SIZE {
            neat_mgr.population[agent.agent_id].fitness = agent.accumulated_reward;
        }
        commands.entity(ent).despawn_recursive();
    }
    
    // Evolve to the next generation
    neat_mgr.evolve();

    for ent in frag_q.iter() { commands.entity(ent).despawn(); }
    for ent in wheel_q.iter() { commands.entity(ent).despawn(); }
    for ent in projectile_q.iter() { commands.entity(ent).despawn_recursive(); }
}

pub fn fragment_lifetime_system(
    mut commands: Commands,
    mut frag_q:   Query<(Entity, &mut FragmentMarker)>,
    time:         Res<Time>,
) {
    for (ent, mut frag) in frag_q.iter_mut() {
        frag.lifetime -= time.delta_seconds();
        if frag.lifetime <= 0.0 { commands.entity(ent).despawn(); }
    }
}

pub fn on_enter_battle(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GravityScale, &mut CarAgent, &mut RigidBody), With<CarMarker>>,
    mut wheel_q: Query<(Entity, &mut GravityScale, &mut RigidBody, Option<&crate::physics::car_controller::PendingJoint>), (With<crate::physics::car_controller::WheelMarker>, Without<CarMarker>)>,
    mut fire_events: EventWriter<crate::weapons::FireWeaponEvent>,
    weapons_q: Query<&crate::weapons::MountedWeapons>,
    speed_state: Res<crate::ui::hud::SimSpeedState>,
    mut time: ResMut<Time<Virtual>>,
) {
    let speed = crate::ui::hud::SPEED_STEPS[speed_state.speed_idx];
    time.set_relative_speed(speed);

    for (w_ent, mut w_grav, mut w_rb, pending) in wheel_q.iter_mut() {
        w_grav.0 = 1.0;
        *w_rb = RigidBody::Dynamic;
        if let Some(p) = pending {
            commands.entity(w_ent)
                .insert(ImpulseJoint::new(p.parent, p.joint.clone()))
                .insert(Friction { coefficient: 1.5, combine_rule: CoefficientCombineRule::Max })
                .remove::<crate::physics::car_controller::PendingJoint>();
        }
    }
    for (ent, mut grav, mut agent, mut rb) in q.iter_mut() {
        grav.0 = 1.0;
        *rb = RigidBody::Dynamic;
        // Dummy reward and shot
        agent.accumulated_reward += 50.0;
        if let Ok(weapons) = weapons_q.get(ent) {
            if let Some(&w_ent) = weapons.weapon_entities.first() {
                // Fire straight ahead
                fire_events.send(crate::weapons::FireWeaponEvent { weapon_entity: w_ent, aim_target: Vec3::Z * 1000.0 });
            }
        }
    }
}

pub fn activate_cars_in_battle(
    mut commands: Commands,
    mut q: Query<(Entity, &mut GravityScale, &mut RigidBody), With<CarMarker>>,
    mut wheel_q: Query<(Entity, &mut GravityScale, &mut RigidBody, Option<&crate::physics::car_controller::PendingJoint>), (With<crate::physics::car_controller::WheelMarker>, Without<CarMarker>)>,
) {
    for (w_ent, mut w_grav, mut w_rb, pending) in wheel_q.iter_mut() {
        if *w_rb == RigidBody::Fixed {
            w_grav.0 = 1.0;
            *w_rb = RigidBody::Dynamic;
            if let Some(p) = pending {
                commands.entity(w_ent)
                    .insert(ImpulseJoint::new(p.parent, p.joint.clone()))
                    .insert(Friction { coefficient: 1.5, combine_rule: CoefficientCombineRule::Max })
                    .remove::<crate::physics::car_controller::PendingJoint>();
            }
        }
    }
    for (_ent, mut grav, mut rb) in q.iter_mut() {
        if *rb == RigidBody::Fixed {
            grav.0 = 1.0;
            *rb = RigidBody::Dynamic;
        }
    }
}

pub fn platform_fitness_system(
    mut agent_q: Query<(Entity, &mut CarAgent, &GlobalTransform)>,
    platform_q: Query<(&HexPlatform, &GlobalTransform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    
    // For each platform, find the agent with the highest accumulated_reward
    for (platform, platform_gtf) in platform_q.iter() {
        let plat_pos = platform_gtf.translation();
        
        let mut best_agent_ent = None;
        let mut max_reward = f32::NEG_INFINITY;
        
        for (ent, agent, agent_gtf) in agent_q.iter() {
            let agent_pos = agent_gtf.translation();
            let dx = agent_pos.x - plat_pos.x;
            let dz = agent_pos.z - plat_pos.z;
            let dy = (agent_pos.y - plat_pos.y).abs();
            
            if dx*dx + dz*dz < 15.0 * 15.0 && dy < 10.0 {
                if agent.accumulated_reward > max_reward {
                    max_reward = agent.accumulated_reward;
                    best_agent_ent = Some(ent);
                }
            }
        }
        
        if let Some(best_ent) = best_agent_ent {
            if let Ok((_, mut best_agent, _)) = agent_q.get_mut(best_ent) {
                if best_agent.accumulated_reward >= platform.min_reward {
                    best_agent.accumulated_reward += platform.fitness_boost * dt;
                }
            }
        }
    }
}
