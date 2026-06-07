// weapons/mod.rs — WeaponsPlugin + shared weapon types
//
// Bevy 0.18 · bevy_rapier3d 0.34

pub mod homing;
pub mod mine;
pub mod projectile;
pub mod systems;

use bevy::prelude::*;

// ─────────────────────────────────────────────────────────────────────────────
// Re-exports consumed by other crates
// ─────────────────────────────────────────────────────────────────────────────
pub use systems::{
    FireWeaponEvent,
    fire_weapon_system,
    weapon_cooldown_system,
    sticky_bomb_system,
    drill_contact_system,
    emp_decay_system,
};
pub use projectile::{ProjectileHitEvent, projectile_lifetime_system, projectile_collision_system};
pub use homing::homing_system;
pub use mine::{mine_arm_system, mine_trigger_system};

// ─────────────────────────────────────────────────────────────────────────────
// WeaponType — 16 variants (discriminants 0..=15)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[repr(u8)]
pub enum WeaponType {
    Rpg              = 0,
    Minigun          = 1,
    SelfAimingTurret = 2,
    Flamethrower     = 3,
    EmpCannon        = 4,
    LaserBeam        = 5,
    GrenadeLauncher  = 6,
    Shotgun          = 7,
    SniperRailGun    = 8,
    HomingMissile    = 9,
    MineLayer        = 10,
    TeslaCoil        = 11,
    PlasmaCannon     = 12,
    Mortar           = 13,
    StickyBomb       = 14,
    Drill            = 15,
}

impl WeaponType {
    /// Base cooldown in seconds between shots.
    pub fn base_cooldown(self) -> f32 {
        match self {
            WeaponType::Rpg              => 1.5,
            WeaponType::Minigun          => 0.05,
            WeaponType::SelfAimingTurret => 0.6,
            WeaponType::Flamethrower     => 0.0,  // continuous
            WeaponType::EmpCannon        => 5.0,
            WeaponType::LaserBeam        => 0.0,  // continuous
            WeaponType::GrenadeLauncher  => 1.2,
            WeaponType::Shotgun          => 0.9,
            WeaponType::SniperRailGun    => 2.5,
            WeaponType::HomingMissile    => 2.0,
            WeaponType::MineLayer        => 2.0,
            WeaponType::TeslaCoil        => 0.8,
            WeaponType::PlasmaCannon     => 2.0,
            WeaponType::Mortar           => 1.8,
            WeaponType::StickyBomb       => 1.5,
            WeaponType::Drill            => 0.0,  // passive
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0  => Self::Rpg,
            1  => Self::Minigun,
            2  => Self::SelfAimingTurret,
            3  => Self::Flamethrower,
            4  => Self::EmpCannon,
            5  => Self::LaserBeam,
            6  => Self::GrenadeLauncher,
            7  => Self::Shotgun,
            8  => Self::SniperRailGun,
            9  => Self::HomingMissile,
            10 => Self::MineLayer,
            11 => Self::TeslaCoil,
            12 => Self::PlasmaCannon,
            13 => Self::Mortar,
            14 => Self::StickyBomb,
            _  => Self::Drill,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WeaponState — per-weapon-entity component
// ─────────────────────────────────────────────────────────────────────────────

/// Attached to each weapon entity.
#[derive(Component, Debug, Reflect)]
pub struct WeaponState {
    /// Time remaining until the weapon can fire again (seconds).
    pub cooldown:    f32,
    /// Which weapon this entity represents.
    pub weapon_type: WeaponType,
    /// Voxel grid position of the weapon block [x, y, z].
    pub grid_pos:    [usize; 3],
}

impl WeaponState {
    pub fn new(weapon_type: WeaponType, grid_pos: [usize; 3]) -> Self {
        Self { cooldown: 0.0, weapon_type, grid_pos }
    }

    pub fn is_ready(&self) -> bool {
        self.cooldown <= 0.0
    }

    pub fn reset_cooldown(&mut self) {
        self.cooldown = self.weapon_type.base_cooldown();
    }

    /// Returns cooldown as a fraction of the base cooldown in [0, 1],
    /// where 0 = fully ready and 1 = just fired.
    pub fn cooldown_frac(&self) -> f32 {
        let base = self.weapon_type.base_cooldown();
        if base <= 0.0 { return 0.0; }
        (self.cooldown / base).clamp(0.0, 1.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MountedWeapons — attached to car root entity
// ─────────────────────────────────────────────────────────────────────────────

/// Holds the entity IDs of all weapon entities mounted on this car.
#[derive(Component, Debug, Default, Reflect)]
pub struct MountedWeapons {
    pub weapon_entities: Vec<Entity>,
}

// ─────────────────────────────────────────────────────────────────────────────
// EmpDisabledTimer — applied to cars hit by EMP cannon
// ─────────────────────────────────────────────────────────────────────────────

/// Seconds of EMP disable remaining. Removed when it reaches zero.
#[derive(Component, Debug, Reflect)]
pub struct EmpDisabledTimer(pub f32);

// ─────────────────────────────────────────────────────────────────────────────
// WeaponsPlugin
// ─────────────────────────────────────────────────────────────────────────────

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app
            // ── Events ────────────────────────────────────────────────────
            .add_event::<FireWeaponEvent>()
            .add_event::<ProjectileHitEvent>()
            // ── Reflect ───────────────────────────────────────────────────
            .register_type::<WeaponState>()
            .register_type::<MountedWeapons>()
            .register_type::<EmpDisabledTimer>()
            // ── Update systems ────────────────────────────────────────────
            .add_systems(Update, (
                weapon_cooldown_system,
                fire_weapon_system.after(weapon_cooldown_system),
                projectile_lifetime_system,
                projectile_collision_system,
                sticky_bomb_system,
                drill_contact_system,
                emp_decay_system,
            ))
            // ── FixedUpdate systems ───────────────────────────────────────
            .add_systems(FixedUpdate, (
                homing_system,
                mine_arm_system,
                mine_trigger_system,
            ));
    }
}
