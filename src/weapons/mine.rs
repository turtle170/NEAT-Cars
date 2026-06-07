// weapons/mine.rs — MineState component, arm system, trigger system
//
// Bevy 0.15 · bevy_rapier3d 0.28

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::battle::damage::SplashDamageRequest;
use crate::voxel::grid::VoxelGrid;

// ─────────────────────────────────────────────────────────────────────────────
// MineState component
// ─────────────────────────────────────────────────────────────────────────────

/// Attached to every mine entity spawned by the MineLayer weapon.
#[derive(Component, Debug, Reflect)]
pub struct MineState {
    /// Whether the mine is armed and ready to detonate.
    pub armed:      bool,
    /// Countdown until the mine becomes armed (seconds).
    pub arm_timer:  f32,
    /// Base damage on detonation.
    pub damage:     f32,
    /// Splash radius in metres.
    pub radius:     f32,
    /// Car entity that placed this mine (for kill attribution).
    pub attacker:   Entity,
}

impl MineState {
    pub fn new(attacker: Entity) -> Self {
        Self {
            armed:     false,
            arm_timer: 1.5,
            damage:    80.0,
            radius:    3.0,
            attacker,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: mine_arm_system (FixedUpdate)
// ─────────────────────────────────────────────────────────────────────────────

/// Counts down the arming delay; marks the mine armed once it expires.
pub fn mine_arm_system(
    time:     Res<Time>,
    mut mine_q: Query<&mut MineState>,
) {
    let dt = time.delta_seconds();
    for mut mine in mine_q.iter_mut() {
        if !mine.armed {
            mine.arm_timer -= dt;
            if mine.arm_timer <= 0.0 {
                mine.armed = true;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// System: mine_trigger_system (FixedUpdate)
// ─────────────────────────────────────────────────────────────────────────────

/// Watches Rapier collision/sensor events.  When an armed mine sensor
/// intersects with any entity that also has a VoxelGrid (i.e. an enemy car),
/// it emits a SplashDamageRequest and despawns itself.
pub fn mine_trigger_system(
    mut commands:       Commands,
    mut collision_evts: EventReader<CollisionEvent>,
    mine_q:             Query<(&MineState, &GlobalTransform)>,
    voxel_q:            Query<&VoxelGrid>,
    mut splash_req:     EventWriter<SplashDamageRequest>,
) {
    for ev in collision_evts.read() {
        let (e1, e2) = match ev {
            CollisionEvent::Started(a, b, _) => (*a, *b),
            CollisionEvent::Stopped(_, _, _) => continue,
        };

        // Determine which entity is the mine and which is the intruder.
        let (mine_ent, other_ent) = if mine_q.contains(e1) {
            (e1, e2)
        } else if mine_q.contains(e2) {
            (e2, e1)
        } else {
            continue;
        };

        let Ok((mine, mine_gtf)) = mine_q.get(mine_ent) else { continue };

        // Only trigger if armed.
        if !mine.armed { continue; }

        // Only trigger against VoxelGrid entities (car bodies).
        if !voxel_q.contains(other_ent) { continue; }

        // Don't trigger against the car that placed the mine.
        if other_ent == mine.attacker { continue; }

        let world_pos = mine_gtf.translation();

        splash_req.send(SplashDamageRequest {
            world_pos,
            radius:      mine.radius,
            base_damage: mine.damage,
            attacker:    mine.attacker,
        });

        commands.entity(mine_ent).despawn();
    }
}
