// weapons/homing.rs — HomingTarget component + homing guidance system
//
// Bevy 0.15 · bevy_rapier3d 0.28

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Components
// ─────────────────────────────────────────────────────────────────────────────

/// Attached to homing-missile projectile entities.
/// `target` is the enemy car entity the missile should pursue.
#[derive(Component, Debug, Clone, Reflect)]
pub struct HomingTarget {
    pub target: Entity,
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const HOMING_SPEED:     f32 = 14.0;          // m/s
const HOMING_TURN_RATE: f32 = 90.0_f32 * (std::f32::consts::PI / 180.0); // rad/s

// ─────────────────────────────────────────────────────────────────────────────
// System: homing_system (FixedUpdate)
// ─────────────────────────────────────────────────────────────────────────────

/// Steers homing missiles toward their locked target.
/// Rotates the velocity vector at HOMING_TURN_RATE °/s and keeps speed at
/// HOMING_SPEED m/s.  Despawns the HomingTarget component if the target entity
/// no longer exists (missile flies straight until lifetime expires).
pub fn homing_system(
    time:        Res<Time>,
    mut missile_q: Query<(Entity, &mut Velocity, &GlobalTransform, &HomingTarget)>,
    target_q:    Query<&GlobalTransform, Without<HomingTarget>>,
    mut commands: Commands,
) {
    let dt = time.delta_seconds();

    for (ent, mut vel, missile_gtf, homing) in missile_q.iter_mut() {
        // If the target is gone, remove HomingTarget so the missile flies straight.
        let Ok(target_gtf) = target_q.get(homing.target) else {
            commands.entity(ent).remove::<HomingTarget>();
            continue;
        };

        let missile_pos = missile_gtf.translation();
        let target_pos  = target_gtf.translation();

        let desired_dir = (target_pos - missile_pos).normalize_or_zero();
        if desired_dir == Vec3::ZERO {
            continue;
        }

        let current_dir = vel.linvel.normalize_or_zero();
        if current_dir == Vec3::ZERO {
            // Cold start: just aim directly.
            vel.linvel = desired_dir * HOMING_SPEED;
            continue;
        }

        // Slerp current direction toward desired direction, clamped by turn rate.
        let max_angle = HOMING_TURN_RATE * dt;
        let angle     = current_dir.angle_between(desired_dir).min(max_angle);

        // Rodrigues rotation around the axis perpendicular to both vectors.
        let new_dir = if angle < 1e-5 {
            desired_dir
        } else {
            let axis = current_dir.cross(desired_dir).normalize_or_zero();
            if axis == Vec3::ZERO {
                desired_dir
            } else {
                Quat::from_axis_angle(axis, angle) * current_dir
            }
        };

        vel.linvel = new_dir.normalize_or_zero() * HOMING_SPEED;
    }
}
