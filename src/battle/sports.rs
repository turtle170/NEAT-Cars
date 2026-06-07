use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::ai::agent::CarAgent;
use crate::battle::manager::CarMarker;

#[derive(Component)]
pub struct Football {
    pub last_touch: Option<Entity>,
}

#[derive(Component)]
pub struct GoalNet {
    pub team_color: Color,
}

#[derive(Component, Clone)]
pub enum DiscusState {
    Idle,
    Held { owner: Entity, spin: f32 },
    Thrown { owner: Entity },
}

pub fn football_system(
    mut collision_events: EventReader<CollisionEvent>,
    mut football_q: Query<(Entity, &mut Football, &mut Transform, &mut Velocity)>,
    car_q: Query<(Entity, &CarMarker)>,
    net_q: Query<&GoalNet>,
    mut agent_q: Query<&mut CarAgent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Find if one is football
            let (fb_ent, other_ent) = if football_q.contains(*e1) { (*e1, *e2) } else if football_q.contains(*e2) { (*e2, *e1) } else { continue; };
            
            let Ok((_, mut fb, mut tf, mut vel)) = football_q.get_mut(fb_ent) else { continue; };

            // Is the other a Car?
            if let Ok((car_ent, _)) = car_q.get(other_ent) {
                fb.last_touch = Some(car_ent);
            }

            // Is the other a Goal Net?
            if let Ok(net) = net_q.get(other_ent) {
                // GOAL SCORED!
                if let Some(scorer) = fb.last_touch {
                    if let Ok(mut agent) = agent_q.get_mut(scorer) {
                        agent.accumulated_reward += 10000.0; // HUGE REWARD!
                    }
                }
                
                // Sparks!
                for _ in 0..50 {
                    use rand::Rng;
                    let dir = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.5..2.0), rng.gen_range(-1.0..1.0)).normalize();
                    let speed = rng.gen_range(20.0..50.0);
                    commands.spawn((
                        PbrBundle {
                            mesh: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
                            material: materials.add(StandardMaterial {
                                base_color: net.team_color,
                                emissive: Color::rgb(net.team_color.r()*20., net.team_color.g()*20., net.team_color.b()*20.),
                                ..default()
                            }),
                            transform: Transform::from_translation(tf.translation),
                            ..default()
                        },
                        RigidBody::Dynamic,
                        Collider::cuboid(0.25, 0.25, 0.25),
                        Velocity { linvel: dir * speed, angvel: Vec3::splat(10.) },
                        CollisionGroups::new(Group::from_bits_truncate(1 << 31), Group::from_bits_truncate(!(1 << 31))),
                        crate::Spark { timer: Timer::from_seconds(rng.gen_range(2.0..4.0), TimerMode::Once) },
                    ));
                }

                // Reset ball
                tf.translation = Vec3::new(0.0, 20.0, 0.0);
                vel.linvel = Vec3::ZERO;
                vel.angvel = Vec3::ZERO;
                fb.last_touch = None;
            }
        }
    }
}

pub fn discus_system(
    mut collision_events: EventReader<CollisionEvent>,
    mut discus_q: Query<(Entity, &mut DiscusState, &mut Transform, &mut Velocity)>,
    car_q: Query<(Entity, &CarMarker, &Transform, &crate::physics::car_controller::CarInput), Without<DiscusState>>,
    mut agent_q: Query<&mut CarAgent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    let mut rng = rand::thread_rng();

    // 1. Handle Pickups and Hits via Collisions
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            let (disc_ent, other_ent) = if discus_q.contains(*e1) { (*e1, *e2) } else if discus_q.contains(*e2) { (*e2, *e1) } else { continue; };
            
            let Ok((_, mut state, tf, _)) = discus_q.get_mut(disc_ent) else { continue; };
            let Ok((car_ent, _, _, _)) = car_q.get(other_ent) else { continue; };

            let new_state = match state.clone() {
                DiscusState::Idle => {
                    // Pick up!
                    Some(DiscusState::Held { owner: car_ent, spin: 0.0 })
                },
                DiscusState::Thrown { owner } => {
                    if owner != car_ent {
                        // HIT! Reward thrower HUGE
                        if let Ok(mut agent) = agent_q.get_mut(owner) {
                            agent.accumulated_reward += 5000.0;
                        }

                        // Generate sparks
                        for _ in 0..20 {
                            use rand::Rng;
                            let dir = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.0..1.0), rng.gen_range(-1.0..1.0)).normalize();
                            commands.spawn((
                                PbrBundle {
                                    mesh: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
                                    material: materials.add(StandardMaterial {
                                        base_color: Color::rgb(1.0, 0.5, 0.0),
                                        emissive: Color::rgb(30.0, 15.0, 0.0),
                                        ..default()
                                    }),
                                    transform: Transform::from_translation(tf.translation),
                                    ..default()
                                },
                                RigidBody::Dynamic,
                                Collider::cuboid(0.2, 0.2, 0.2),
                                Velocity { linvel: dir * rng.gen_range(15.0..30.0), angvel: Vec3::splat(5.) },
                                CollisionGroups::new(Group::from_bits_truncate(1 << 31), Group::from_bits_truncate(!(1 << 31))),
                                crate::Spark { timer: Timer::from_seconds(rng.gen_range(1.5..2.5), TimerMode::Once) },
                            ));
                        }

                        // Discus falls down
                        Some(DiscusState::Idle)
                    } else { None }
                },
                _ => None,
            };

            if let Some(ns) = new_state {
                *state = ns;
            }
        }
    }

    // 2. Handle Held Discus (Spinning, Tracking, Throwing, Dropping)
    for (_disc_ent, mut state, mut tf, mut vel) in discus_q.iter_mut() {
        let mut transition = None;
        if let DiscusState::Held { owner, spin } = &mut *state {
            if let Ok((_, _, car_tf, _)) = car_q.get(*owner) {
                // Snap to top of car
                tf.translation = car_tf.translation + Vec3::new(0.0, 4.0, 0.0);
                
                if let Ok(agent) = agent_q.get(*owner) {
                    if agent.current_action.len() >= 9 {
                        let action_spin = agent.current_action[7];
                        let action_throw = agent.current_action[8];

                        // Spin
                        if action_spin > 0.5 {
                            *spin += 10.0 * dt;
                            tf.rotate_y(5.0 * dt + *spin * dt);
                        }

                        // Throw / Release
                        if action_throw > 0.5 {
                            let forward = car_tf.forward();
                            let speed = 20.0 + (*spin * 5.0); // Spin makes it faster
                            vel.linvel = forward * speed;
                            vel.angvel = Vec3::new(0.0, 20.0, 0.0);
                            tf.translation += forward * 5.0; // spawn slightly ahead
                            transition = Some(DiscusState::Thrown { owner: *owner });
                        }
                        // Drop (if holding but spinning is 0 and throw is pressed? Or just drop if spinning is disabled but drop is pressed)
                        // Actually, if we just release with low spin, it drops natively.
                    }
                }
            } else {
                // Owner died? Drop it.
                transition = Some(DiscusState::Idle);
            }
        }

        if let Some(ns) = transition {
            *state = ns;
        }

        // 3. Gravity adjustment for Thrown (keep it hovering a bit or let it fall? Sensor means it will fall through ground if not fixed)
        // Wait, if it's a Sensor, it doesn't collide with ground!
        // The user didn't say Sensor, they said "discuses around the map that the car can go near and pick up". 
        // We will make them Dynamic bodies with `CollisionGroups` that hit cars, but what about the floor?
        // Actually, if it's Idle it rests on the floor. If it's thrown, we can apply GravityScale(0.0) so it glides!
        if let DiscusState::Thrown { .. } = *state {
            // Apply zero gravity? No, we can just apply a small upward force or `GravityScale`.
            // But we don't have GravityScale component directly accessible here unless we query it.
            // Let's just set Y velocity to zero and lock Y translation to stay flat.
            vel.linvel.y = 0.0;
        }
    }
}
