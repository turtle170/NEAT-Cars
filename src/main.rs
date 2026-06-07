#![allow(dead_code)]
#![allow(unused_imports)]
// main.rs — NEAT Cars  (Bevy 0.13 · bevy_rapier3d 0.25)

mod voxel;
mod physics;
mod weapons;
mod ai;
mod battle;
mod camera;
mod ui;
mod assets_loader;

#[derive(Component)]
pub struct WallMarker;

#[derive(Component)]
pub struct Spark { pub timer: Timer }

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_obj::ObjPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.01, 0.01, 0.02)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "NEAT Cars".into(),
                resolution: (1920_f32, 1080_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(RapierConfiguration {
            timestep_mode: TimestepMode::Fixed { dt: 1.0 / 180.0, substeps: 1 },
            ..default()
        })
        .add_plugins(ObjPlugin)
        .add_plugins(voxel::VoxelPlugin)
        .add_plugins(physics::PhysicsPlugin)
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(ai::AiPlugin)
        .add_plugins(battle::BattlePlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(assets_loader::AssetsPlugin)
        .add_systems(Startup, setup_arena)
        .add_systems(Update, spark_system.run_if(in_state(crate::battle::manager::EpisodeState::Battle)))
        .add_systems(Update, (ribbon_trail_setup_system, ribbon_trail_update_system))
        .run();
}

#[derive(Component)]
pub struct RibbonTrail {
    pub car_entity: Entity,
    pub points: std::collections::VecDeque<(Vec3, f32)>, // pos, time_added
    pub width: f32,
    pub lifetime: f32,
}

#[derive(Component)]
pub struct RibbonTrailTracker;

fn ribbon_trail_setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    car_q: Query<Entity, (With<crate::battle::manager::CarMarker>, Without<RibbonTrailTracker>)>,
) {
    for car_ent in car_q.iter() {
        let mat = materials.add(StandardMaterial {
            base_color: Color::rgb(0.0, 1.0, 1.0),
            emissive: Color::rgb(0.0, 200.0, 200.0), // Neon cyan glow
            alpha_mode: AlphaMode::Add,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleStrip, 
            bevy::render::render_asset::RenderAssetUsages::default()
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.,0.,0.], [0.,0.,0.], [0.,0.,0.]]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.,1.,0.], [0.,1.,0.], [0.,1.,0.]]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.,0.], [0.,0.], [0.,0.]]);
        let mesh_handle = meshes.add(mesh);
        
        commands.spawn((
            PbrBundle {
                mesh: mesh_handle,
                material: mat,
                ..default()
            },
            bevy::render::view::NoFrustumCulling,
            RibbonTrail {
                car_entity: car_ent,
                points: std::collections::VecDeque::new(),
                width: 1.5,
                lifetime: 0.4, // half second trail
            }
        ));
        
        commands.entity(car_ent).insert(RibbonTrailTracker);
    }
}

fn ribbon_trail_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut trail_q: Query<(Entity, &mut RibbonTrail, &Handle<Mesh>)>,
    car_q: Query<(&GlobalTransform, &Velocity), With<crate::battle::manager::CarMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let now = time.elapsed_seconds() as f32;
    for (trail_ent, mut trail, mesh_handle) in trail_q.iter_mut() {
        // Add current pos to trail if car is alive and moving
        if let Ok((gtf, vel)) = car_q.get(trail.car_entity) {
            if vel.linvel.length() > 2.0 {
                let pos = gtf.translation() - gtf.forward() * 3.5 + gtf.up() * 0.5;
                trail.points.push_back((pos, now));
            }
        }
        
        // Remove old points
        while let Some(&(_, t)) = trail.points.front() {
            if now - t > trail.lifetime {
                trail.points.pop_front();
            } else {
                break;
            }
        }
        
        // If car is dead and all points faded out, destroy the trail entity
        if trail.points.is_empty() && car_q.get(trail.car_entity).is_err() {
            commands.entity(trail_ent).despawn();
            continue;
        }
        
        // Update mesh
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut uvs: Vec<[f32; 2]> = Vec::new();
            let mut normals: Vec<[f32; 3]> = Vec::new();
            
            if trail.points.len() >= 2 {
                for i in 0..trail.points.len() {
                    let (p, t) = trail.points[i];
                    let age = now - t;
                    let alpha = 1.0 - (age / trail.lifetime).clamp(0.0, 1.0);
                    let w = trail.width * alpha; // taper off
                    
                    let dir = if i < trail.points.len() - 1 {
                        (trail.points[i+1].0 - p).normalize_or_zero()
                    } else {
                        (p - trail.points[i-1].0).normalize_or_zero()
                    };
                    
                    let right = dir.cross(Vec3::Y).normalize_or_zero() * (w * 0.5);
                    
                    positions.push((p - right).to_array());
                    positions.push((p + right).to_array());
                    
                    normals.push([0.0, 1.0, 0.0]);
                    normals.push([0.0, 1.0, 0.0]);
                    
                    let v = i as f32 / (trail.points.len() - 1) as f32;
                    uvs.push([0.0, v]);
                    uvs.push([1.0, v]);
                }
            } else {
                positions = vec![[0.,0.,0.], [0.,0.,0.], [0.,0.,0.]];
                normals = vec![[0.,1.,0.], [0.,1.,0.], [0.,1.,0.]];
                uvs = vec![[0.,0.], [0.,0.], [0.,0.]];
            }
            
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
    }
}

fn spark_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut collision_events: EventReader<CollisionEvent>,
    transforms: Query<&Transform>,
    mut query: Query<(Entity, &mut Spark)>,
    car_query: Query<(), With<crate::battle::manager::CarMarker>>,
    mut impulse_query: Query<&mut ExternalImpulse>,
) {
    let mut rng = rand::thread_rng();
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Prevent sparks from spawning more sparks (infinite loop!)
            if query.get(*e1).is_ok() || query.get(*e2).is_ok() { continue; }

            // Only spawn sparks when CARS COLLIDE WITH CARS
            if car_query.get(*e1).is_err() || car_query.get(*e2).is_err() { continue; }

            let pos = transforms.get(*e1).or(transforms.get(*e2)).map(|t| t.translation).unwrap_or(Vec3::ZERO);
            
            // Only spawn if above a certain height so we don't spam from floor sinking
            if pos.y < 0.0 { continue; }

            if let Ok([mut imp1, mut imp2]) = impulse_query.get_many_mut([*e1, *e2]) {
                let p1 = transforms.get(*e1).unwrap().translation;
                let p2 = transforms.get(*e2).unwrap().translation;
                let dir = (p1 - p2).normalize_or_zero();
                imp1.impulse += dir * 80000.0;
                imp2.impulse -= dir * 80000.0;
            }

            for _ in 0..40 {
                use rand::Rng;
                let dir = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.5..2.0), rng.gen_range(-1.0..1.0)).normalize();
                let speed = rng.gen_range(20.0..60.0);
                
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Cuboid::new(0.4, 0.4, 2.0).mesh()),
                        material: materials.add(StandardMaterial {
                            base_color: Color::rgb(10.0, 10.0, 5.0),
                            emissive: Color::rgb(2000.0, 2000.0, 800.0), // EXTREMELY BRIGHT trail sparks
                            ..default()
                        }),
                        transform: Transform::from_translation(pos).looking_to(dir, Vec3::Y),
                        ..default()
                    },
                    RigidBody::Dynamic,
                    Collider::cuboid(0.2, 0.2, 1.0),
                    Velocity {
                        linvel: dir * speed,
                        angvel: Vec3::ZERO,
                    },
                    // Sparks group: won't collide with other sparks to save performance
                    CollisionGroups::new(Group::from_bits_truncate(1 << 31), Group::from_bits_truncate(!(1 << 31))),
                    Spark { timer: Timer::from_seconds(rng.gen_range(0.5..1.0), TimerMode::Once) },
                ));
            }
        }
    }
    for (entity, mut spark) in query.iter_mut() {
        spark.timer.tick(time.delta());
        if spark.timer.finished() { commands.entity(entity).despawn(); }
    }
}

fn setup_arena(
    mut commands: Commands,
    mut meshes:   ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(0.5, 0.7, 1.0),
            illuminance: 10_000.0, // dark atmosphere
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -1.15, 0.3, 0.0,
        )),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::rgb(0.0, 0.5, 1.0),
        brightness: 150.0,
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.05, 0.05, 0.06),
        emissive: Color::rgb(0.05, 0.05, 0.08), // slight map glow
        metallic: 0.1,
        perceptual_roughness: 0.8,
        ..default()
    });

    // Floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(600.0, 20.0, 1200.0)),
            material: floor_mat,
            transform: Transform::from_translation(Vec3::new(0.0, -10.0, 0.0)),
            ..default()
        },
        RigidBody::Fixed,
        Collider::cuboid(300.0, 10.0, 600.0),
        Friction { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
        Restitution { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
        ActiveEvents::COLLISION_EVENTS,
        WallMarker,
    ));

    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.03, 0.03, 0.04),
        metallic: 0.1,
        perceptual_roughness: 0.8,
        ..default()
    });

    let walls: &[(Vec3, Quat, Vec3)] = &[
        (Vec3::new( 0., 6.,  600.), Quat::IDENTITY,                          Vec3::new(300., 6., 0.5)),
        (Vec3::new( 0., 6., -600.), Quat::IDENTITY,                          Vec3::new(300., 6., 0.5)),
        (Vec3::new( 300., 6.,  0.), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), Vec3::new(600., 6., 0.5)),
        (Vec3::new(-300., 6.,  0.), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), Vec3::new(600., 6., 0.5)),
    ];

    for (pos, rot, half) in walls {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(half.x * 2., half.y * 2., half.z * 2.)),
                material: wall_mat.clone(),
                transform: Transform { translation: *pos, rotation: *rot, ..default() },
                ..default()
            },
            RigidBody::Fixed,
            Collider::cuboid(half.x, half.y, half.z),
            Friction { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            Restitution { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            ActiveEvents::COLLISION_EVENTS,
            WallMarker,
        ));
    }

    // Invisible Borders (to prevent cars flying out)
    // Very thick walls to prevent high-speed tunneling
    let invis_walls = [
        (Vec3::new( 0., 150.,  200.), Vec3::new(250., 150., 50.)),
        (Vec3::new( 0., 150., -200.), Vec3::new(250., 150., 50.)),
        (Vec3::new( 200., 150.,  0.), Vec3::new(50., 150., 250.)),
        (Vec3::new(-200., 150.,  0.), Vec3::new(50., 150., 250.)),
        (Vec3::new( 0., 350.,   0.), Vec3::new(250., 50., 250.)), // Ceiling
    ];
    for (pos, half) in invis_walls {
        commands.spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            RigidBody::Fixed,
            Collider::cuboid(half.x, half.y, half.z),
            Friction { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            Restitution { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            ActiveEvents::COLLISION_EVENTS,
            WallMarker,
        ));
    }

    // Hexagon Platforms
    let hex_mesh = meshes.add(Cylinder::new(15.0, 1.0).mesh().resolution(6));
    let hex_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.02, 0.1, 0.05),
        emissive: Color::rgb(0.0, 2.5, 1.0), // stronger glow
        ..default()
    });

    let platforms = [
        (Vec3::new(0.0, 40.0, 0.0), 300.0, 20.0),
        (Vec3::new(25.0, 80.0, -25.0), 600.0, 40.0),
        (Vec3::new(-25.0, 120.0, 25.0), 1000.0, 100.0),
    ];

    for (pos, min_reward, boost) in platforms {
        commands.spawn((
            PbrBundle {
                mesh: hex_mesh.clone(),
                material: hex_mat.clone(),
                transform: Transform::from_translation(pos),
                ..default()
            },
            RigidBody::Fixed,
            Collider::cylinder(1.0, 15.0),
            Friction { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            Restitution { coefficient: 0.0, combine_rule: CoefficientCombineRule::Min },
            ActiveEvents::COLLISION_EVENTS,
            WallMarker,
            crate::battle::manager::HexPlatform {
                min_reward,
                fitness_boost: boost,
            },
        ));
    }

    // Neon Cyan Grid
    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.0, 1.0, 1.0),
        emissive: Color::rgb(0.0, 5.0, 5.0),
        ..default()
    });
    let mesh_x = meshes.add(Cuboid::new(600.0, 0.1, 0.1));
    let mesh_z = meshes.add(Cuboid::new(0.1, 0.1, 1200.0));
    for i in -120..=120 {
        let p = i as f32 * 5.0;
        commands.spawn(PbrBundle {
            mesh: mesh_x.clone(),
            material: grid_mat.clone(),
            transform: Transform::from_xyz(0.0, 0.05, p),
            ..default()
        });
    }
    for i in -60..=60 {
        let p = i as f32 * 5.0;
        commands.spawn(PbrBundle {
            mesh: mesh_z.clone(),
            material: grid_mat.clone(),
            transform: Transform::from_xyz(p, 0.05, 0.0),
            ..default()
        });
    }

    // Football
    let ball_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(2.0, 2.0, 2.0),
        emissive: Color::rgb(2.0, 2.0, 2.0),
        perceptual_roughness: 0.5,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Sphere::new(8.0).mesh().ico(5).unwrap()),
            material: ball_mat,
            transform: Transform::from_translation(Vec3::new(0.0, 20.0, 0.0)),
            ..default()
        },
        RigidBody::Dynamic,
        Collider::ball(8.0),
        ColliderMassProperties::Mass(200.0), // pushable
        Restitution::coefficient(0.8),
        Damping { linear_damping: 0.5, angular_damping: 0.5 },
        ActiveEvents::COLLISION_EVENTS,
        crate::battle::sports::Football { last_touch: None },
    ));

    // Nets
    let mut spawn_net = |pos: Vec3, rot: Quat, color: Color| {
        let mat = materials.add(StandardMaterial {
            base_color: color, emissive: Color::rgb(color.r()*5., color.g()*5., color.b()*5.), alpha_mode: AlphaMode::Blend, ..default()
        });
        
        let mut builder = commands.spawn((
            SpatialBundle::from_transform(Transform::from_translation(pos).with_rotation(rot)),
            RigidBody::Dynamic,
            ColliderMassProperties::Mass(1000.0),
            Damping { linear_damping: 5.0, angular_damping: 5.0 },
            crate::battle::sports::GoalNet { team_color: color },
        ));
        
        builder.with_children(|parent| {
            // Back wall
            parent.spawn((
                PbrBundle { mesh: meshes.add(Cuboid::new(60., 40., 1.)), material: mat.clone(), transform: Transform::from_xyz(0., 20., -15.), ..default() },
                Collider::cuboid(30., 20., 0.5),
            ));
            // Top wall
            parent.spawn((
                PbrBundle { mesh: meshes.add(Cuboid::new(60., 1., 30.)), material: mat.clone(), transform: Transform::from_xyz(0., 40., 0.), ..default() },
                Collider::cuboid(30., 0.5, 15.),
            ));
            // Left wall
            parent.spawn((
                PbrBundle { mesh: meshes.add(Cuboid::new(1., 40., 30.)), material: mat.clone(), transform: Transform::from_xyz(-30., 20., 0.), ..default() },
                Collider::cuboid(0.5, 20., 15.),
            ));
            // Right wall
            parent.spawn((
                PbrBundle { mesh: meshes.add(Cuboid::new(1., 40., 30.)), material: mat.clone(), transform: Transform::from_xyz(30., 20., 0.), ..default() },
                Collider::cuboid(0.5, 20., 15.),
            ));
        });
    };
    
    // Z is -600 to 600, so nets at 550 and -550
    spawn_net(Vec3::new(0., 0., 500.), Quat::from_rotation_y(std::f32::consts::PI), Color::rgb(1.0, 0.0, 0.0));
    spawn_net(Vec3::new(0., 0., -500.), Quat::IDENTITY, Color::rgb(0.0, 0.0, 1.0));

    // Discuses
    let disc_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.2, 1.0),
        emissive: Color::rgb(8.0, 2.0, 10.0),
        ..default()
    });
    let disc_mesh = meshes.add(Cylinder::new(3.0, 0.5).mesh().resolution(16));
    
    let mut rng = rand::thread_rng();
    for _ in 0..10 {
        use rand::Rng;
        let x = rng.gen_range(-250.0..250.0);
        let z = rng.gen_range(-550.0..550.0);
        commands.spawn((
            PbrBundle {
                mesh: disc_mesh.clone(),
                material: disc_mat.clone(),
                transform: Transform::from_xyz(x, 0.5, z),
                ..default()
            },
            RigidBody::Dynamic,
            Collider::cylinder(0.25, 3.0),
            ActiveEvents::COLLISION_EVENTS,
            crate::battle::sports::DiscusState::Idle,
        ));
    }
}
