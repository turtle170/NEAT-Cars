// weapons/projectile.rs — ProjectileBundle, lifetime + collision systems
//
// Bevy 0.15 · bevy_rapier3d 0.28

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::battle::damage::SplashDamageRequest;

// ─────────────────────────────────────────────────────────────────────────────
// Projectile marker + metadata
// ─────────────────────────────────────────────────────────────────────────────

/// Marker component on all standard projectile entities.
#[derive(Component, Debug)]
pub struct Projectile {
    /// Remaining lifetime in seconds.
    pub lifetime:    f32,
    /// Splash radius on impact (metres).
    pub radius:      f32,
    /// Base damage dealt on impact.
    pub damage:      f32,
    /// Entity of the car that fired this projectile.
    pub attacker:    Entity,
}

// ─────────────────────────────────────────────────────────────────────────────
// ProjectileBundle — everything needed to spawn a projectile
// ─────────────────────────────────────────────────────────────────────────────

/// Helper struct — call `commands.spawn(ProjectileBundle::new(...))`.
/// Consists of a Dynamic rigid body, a small ball collider marked as a Sensor,
/// and initial velocity already set.
#[derive(Bundle)]
pub struct ProjectileBundle {
    pub projectile:      Projectile,
    pub transform:       Transform,
    pub global_transform: GlobalTransform,
    pub visibility:      Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub rigid_body:      RigidBody,
    pub velocity:        Velocity,
    pub collider:        Collider,
    pub sensor:          Sensor,
    pub active_events:   ActiveEvents,
    pub gravity_scale:   GravityScale,
}

impl ProjectileBundle {
    /// * `origin`   – world-space spawn position
    /// * `velocity` – initial world-space velocity
    /// * `damage`   – base damage value
    /// * `radius`   – splash radius in metres
    /// * `lifetime` – seconds until auto-despawn
    /// * `attacker` – entity of the firing car
    /// * `gravity`  – 1.0 = normal gravity, 0.0 = gravity-free (e.g. laser)
    pub fn new(
        origin:   Vec3,
        vel:      Vec3,
        damage:   f32,
        radius:   f32,
        lifetime: f32,
        attacker: Entity,
        gravity:  f32,
    ) -> Self {
        Self {
            projectile: Projectile { lifetime, radius, damage, attacker },
            transform:  Transform::from_translation(origin),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
            rigid_body:   RigidBody::Dynamic,
            velocity:     Velocity::linear(vel),
            collider:     Collider::ball(0.08),
            sensor:       Sensor,
            active_events: ActiveEvents::COLLISION_EVENTS,
            gravity_scale: GravityScale(gravity),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ProjectileHitEvent
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when a projectile hits something.
#[derive(Event, Debug, Clone)]
pub struct ProjectileHitEvent {
    /// Entity of the projectile that hit.
    pub projectile_entity: Entity,
    /// World position of the impact point (approx. projectile translation).
    pub world_pos:         Vec3,
    /// Splash radius of the weapon.
    pub radius:            f32,
    /// Base damage of the weapon.
    pub damage:            f32,
    /// Firing car entity.
    pub attacker:          Entity,
}

// ─────────────────────────────────────────────────────────────────────────────
// System: projectile_lifetime_system
// ─────────────────────────────────────────────────────────────────────────────

/// Ticks down each projectile's lifetime and despawns it when it expires.
pub fn projectile_lifetime_system(
    mut commands:    Commands,
    time:            Res<Time>,
    mut proj_q:      Query<(Entity, &mut Projectile)>,
) {
    let dt = time.delta_seconds();
    for (entity, mut proj) in proj_q.iter_mut() {
        proj.lifetime -= dt;
        if proj.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: projectile_collision_system
// ─────────────────────────────────────────────────────────────────────────────

/// Reacts to Rapier collision events for projectile sensors.
/// Despawns the projectile and emits a SplashDamageRequest.
pub fn projectile_collision_system(
    mut commands:       Commands,
    mut collision_evts: EventReader<CollisionEvent>,
    proj_q:             Query<(&Projectile, &GlobalTransform)>,
    mut splash_req:     EventWriter<SplashDamageRequest>,
    mut hit_events:     EventWriter<ProjectileHitEvent>,
    owner_q:            Query<&crate::physics::car_controller::Owner>,
) {
    for ev in collision_evts.read() {
        let (started, e1, e2) = match ev {
            CollisionEvent::Started(a, b, _) => (true, *a, *b),
            CollisionEvent::Stopped(_, _, _) => continue,
        };
        if !started { continue; }

        // Find which entity is the projectile (if any).
        let (proj_ent, other_ent) = if proj_q.contains(e1) {
            (e1, e2)
        } else if proj_q.contains(e2) {
            (e2, e1)
        } else {
            continue;
        };

        let Ok((proj, gtf)) = proj_q.get(proj_ent) else { continue };

        // Avoid projectile hitting its own attacker or any part owned by the attacker (e.g. wheels, debris)
        if crate::weapons::systems::is_owner_or_part(other_ent, proj.attacker, &owner_q) { continue; }

        let world_pos = gtf.translation();

        splash_req.send(SplashDamageRequest {
            world_pos,
            radius:      proj.radius,
            base_damage: proj.damage,
            attacker:    proj.attacker,
        });

        hit_events.send(ProjectileHitEvent {
            projectile_entity: proj_ent,
            world_pos,
            radius:   proj.radius,
            damage:   proj.damage,
            attacker: proj.attacker,
        });

        commands.entity(proj_ent).despawn();
    }
}
