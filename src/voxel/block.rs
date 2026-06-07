// voxel/block.rs — 22 block types with full stats and special abilities

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Reflect,
)]
#[repr(u8)]
pub enum BlockType {
    #[default]
    Empty = 0,
    Steel = 1,
    Titanium = 2,
    Wood = 3,
    Explosive = 4,
    Shield = 5,
    Rubber = 6,
    CarbonFiber = 7,
    Glass = 8,
    Lead = 9,
    Ceramic = 10,
    ReactiveArmor = 11,
    NanoComposite = 12,
    Magnet = 13,
    Spikes = 14,
    Foam = 15,
    Gold = 16,
    Ice = 17,
    Concrete = 18,
    Mesh = 19,
    PlasmaCore = 20,
    DepletedUranium = 21,
    Honeycomb = 22,
    WheelSpiked = 23,
    WheelBig = 24,
    WheelAero = 25,
    WheelHeavy = 26,
    WheelBouncy = 27,
}

impl BlockType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1  => Self::Steel,
            2  => Self::Titanium,
            3  => Self::Wood,
            4  => Self::Explosive,
            5  => Self::Shield,
            6  => Self::Rubber,
            7  => Self::CarbonFiber,
            8  => Self::Glass,
            9  => Self::Lead,
            10 => Self::Ceramic,
            11 => Self::ReactiveArmor,
            12 => Self::NanoComposite,
            13 => Self::Magnet,
            14 => Self::Spikes,
            15 => Self::Foam,
            16 => Self::Gold,
            17 => Self::Ice,
            18 => Self::Concrete,
            19 => Self::Mesh,
            20 => Self::PlasmaCore,
            21 => Self::DepletedUranium,
            22 => Self::Honeycomb,
            23 => Self::WheelSpiked,
            24 => Self::WheelBig,
            25 => Self::WheelAero,
            26 => Self::WheelHeavy,
            27 => Self::WheelBouncy,
            _  => Self::Empty,
        }
    }

    pub fn is_wheel(&self) -> bool {
        matches!(self, Self::WheelSpiked | Self::WheelBig | Self::WheelAero | Self::WheelHeavy | Self::WheelBouncy)
    }

    pub fn count() -> u8 { 28 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Special Abilities
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum SpecialAbility {
    None,
    Flammable,
    ExplodeOnDeath,
    DamageReduction,
    ProjectileBounce,
    SelfRepair,
    DeflectMetal,
    ContactDamage,
    ReduceFallDamage,
    ReflectLaser,
    Slippery,
    PartialPenetration,
    AdjacencyDamage,
    ResistDeform,
    AbsorbFirstHit,
    Brittle,
}

// ─────────────────────────────────────────────────────────────────────────────
// Block Stats (compile-time const table)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats {
    pub block_type:      BlockType,
    pub display_name:    &'static str,
    pub mass:            f32,
    pub max_health:      f32,
    /// Health fraction below which deformation begins
    pub deform_threshold: f32,
    /// Armor factor: effective_dmg = raw / (1 + armor)
    pub armor:           f32,
    pub special:         SpecialAbility,
    pub color:           [f32; 4],   // RGBA linear
}

impl BlockStats {
    pub fn get(t: BlockType) -> &'static BlockStats {
        &BLOCK_TABLE[t as usize]
    }
}

/// Static lookup table — all 23 entries (index 0 = Empty)
pub static BLOCK_TABLE: &[BlockStats] = &[
    // 0 Empty
    BlockStats { block_type: BlockType::Empty,          display_name: "Empty",            mass: 0.0,  max_health: 0.0,   deform_threshold: 0.0, armor: 0.0, special: SpecialAbility::None,             color: [0.0,  0.0,  0.0,  0.0 ] },
    // 1 Steel
    BlockStats { block_type: BlockType::Steel,          display_name: "Steel",            mass: 5.0,  max_health: 100.0, deform_threshold: 0.5, armor: 0.2, special: SpecialAbility::None,             color: [0.55, 0.60, 0.65, 1.0 ] },
    // 2 Titanium
    BlockStats { block_type: BlockType::Titanium,       display_name: "Titanium",         mass: 8.0,  max_health: 200.0, deform_threshold: 0.4, armor: 0.5, special: SpecialAbility::None,             color: [0.75, 0.80, 0.88, 1.0 ] },
    // 3 Wood
    BlockStats { block_type: BlockType::Wood,           display_name: "Wood",             mass: 2.0,  max_health: 50.0,  deform_threshold: 0.6, armor: 0.0, special: SpecialAbility::Flammable,        color: [0.55, 0.35, 0.15, 1.0 ] },
    // 4 Explosive
    BlockStats { block_type: BlockType::Explosive,      display_name: "Explosive",        mass: 3.0,  max_health: 40.0,  deform_threshold: 0.7, armor: 0.0, special: SpecialAbility::ExplodeOnDeath,   color: [1.0,  0.40, 0.0,  1.0 ] },
    // 5 Shield
    BlockStats { block_type: BlockType::Shield,         display_name: "Shield",           mass: 6.0,  max_health: 150.0, deform_threshold: 0.3, armor: 0.6, special: SpecialAbility::DamageReduction,  color: [0.2,  0.5,  0.9,  1.0 ] },
    // 6 Rubber
    BlockStats { block_type: BlockType::Rubber,         display_name: "Rubber",           mass: 2.0,  max_health: 60.0,  deform_threshold: 0.8, armor: 0.1, special: SpecialAbility::ProjectileBounce, color: [0.15, 0.15, 0.15, 1.0 ] },
    // 7 Carbon Fiber
    BlockStats { block_type: BlockType::CarbonFiber,    display_name: "Carbon Fiber",     mass: 4.0,  max_health: 80.0,  deform_threshold: 0.5, armor: 0.2, special: SpecialAbility::None,             color: [0.10, 0.10, 0.12, 1.0 ] },
    // 8 Glass
    BlockStats { block_type: BlockType::Glass,          display_name: "Glass",            mass: 1.0,  max_health: 20.0,  deform_threshold: 0.9, armor: 0.0, special: SpecialAbility::None,             color: [0.7,  0.9,  1.0,  0.5 ] },
    // 9 Lead
    BlockStats { block_type: BlockType::Lead,           display_name: "Lead",             mass: 12.0, max_health: 120.0, deform_threshold: 0.3, armor: 0.4, special: SpecialAbility::None,             color: [0.35, 0.35, 0.38, 1.0 ] },
    // 10 Ceramic
    BlockStats { block_type: BlockType::Ceramic,        display_name: "Ceramic",          mass: 3.0,  max_health: 90.0,  deform_threshold: 0.1, armor: 0.3, special: SpecialAbility::Brittle,          color: [0.95, 0.92, 0.85, 1.0 ] },
    // 11 Reactive Armor
    BlockStats { block_type: BlockType::ReactiveArmor,  display_name: "Reactive Armor",   mass: 7.0,  max_health: 110.0, deform_threshold: 0.5, armor: 0.7, special: SpecialAbility::AbsorbFirstHit,   color: [0.6,  0.5,  0.2,  1.0 ] },
    // 12 NanoComposite
    BlockStats { block_type: BlockType::NanoComposite,  display_name: "Nano-Composite",   mass: 5.0,  max_health: 130.0, deform_threshold: 0.4, armor: 0.3, special: SpecialAbility::SelfRepair,       color: [0.4,  0.8,  0.6,  1.0 ] },
    // 13 Magnet
    BlockStats { block_type: BlockType::Magnet,         display_name: "Magnet",           mass: 4.0,  max_health: 70.0,  deform_threshold: 0.5, armor: 0.1, special: SpecialAbility::DeflectMetal,     color: [0.7,  0.2,  0.2,  1.0 ] },
    // 14 Spikes
    BlockStats { block_type: BlockType::Spikes,         display_name: "Spikes",           mass: 3.0,  max_health: 60.0,  deform_threshold: 0.6, armor: 0.0, special: SpecialAbility::ContactDamage,    color: [0.5,  0.5,  0.55, 1.0 ] },
    // 15 Foam
    BlockStats { block_type: BlockType::Foam,           display_name: "Foam",             mass: 1.0,  max_health: 30.0,  deform_threshold: 0.9, armor: 0.0, special: SpecialAbility::ReduceFallDamage,  color: [1.0,  0.9,  0.5,  1.0 ] },
    // 16 Gold
    BlockStats { block_type: BlockType::Gold,           display_name: "Gold",             mass: 10.0, max_health: 100.0, deform_threshold: 0.4, armor: 0.2, special: SpecialAbility::ReflectLaser,     color: [1.0,  0.84, 0.0,  1.0 ] },
    // 17 Ice
    BlockStats { block_type: BlockType::Ice,            display_name: "Ice",              mass: 2.0,  max_health: 45.0,  deform_threshold: 0.7, armor: 0.0, special: SpecialAbility::Slippery,         color: [0.8,  0.95, 1.0,  1.0 ] },
    // 18 Concrete
    BlockStats { block_type: BlockType::Concrete,       display_name: "Concrete",         mass: 9.0,  max_health: 160.0, deform_threshold: 0.3, armor: 0.4, special: SpecialAbility::None,             color: [0.6,  0.6,  0.6,  1.0 ] },
    // 19 Mesh
    BlockStats { block_type: BlockType::Mesh,           display_name: "Mesh",             mass: 2.0,  max_health: 55.0,  deform_threshold: 0.6, armor: 0.0, special: SpecialAbility::PartialPenetration,color: [0.5,  0.5,  0.5,  1.0 ] },
    // 20 PlasmaCore
    BlockStats { block_type: BlockType::PlasmaCore,     display_name: "Plasma Core",      mass: 6.0,  max_health: 80.0,  deform_threshold: 0.5, armor: 0.1, special: SpecialAbility::AdjacencyDamage,  color: [0.3,  0.9,  1.0,  1.0 ] },
    // 21 DepletedUranium
    BlockStats { block_type: BlockType::DepletedUranium,display_name: "Depleted Uranium",  mass: 15.0, max_health: 180.0, deform_threshold: 0.2, armor: 0.8, special: SpecialAbility::None,             color: [0.25, 0.30, 0.20, 1.0 ] },
    // 22 Honeycomb
    BlockStats { block_type: BlockType::Honeycomb,      display_name: "Honeycomb",        mass: 3.0,  max_health: 95.0,  deform_threshold: 0.3, armor: 0.3, special: SpecialAbility::ResistDeform,     color: [1.0,  0.75, 0.2,  1.0 ] },
    // 23 WheelSpiked
    BlockStats { block_type: BlockType::WheelSpiked,    display_name: "Spiked Wheel",     mass: 12.0, max_health: 90.0,  deform_threshold: 0.9, armor: 0.1, special: SpecialAbility::ContactDamage,    color: [0.6,  0.1,  0.1,  1.0 ] },
    // 24 WheelBig
    BlockStats { block_type: BlockType::WheelBig,       display_name: "Big Wheel",        mass: 25.0, max_health: 150.0, deform_threshold: 0.9, armor: 0.2, special: SpecialAbility::None,             color: [0.2,  0.2,  0.2,  1.0 ] },
    // 25 WheelAero
    BlockStats { block_type: BlockType::WheelAero,      display_name: "Aero Wheel",       mass: 8.0,  max_health: 60.0,  deform_threshold: 0.9, armor: 0.0, special: SpecialAbility::None,             color: [0.1,  0.5,  0.9,  1.0 ] },
    // 26 WheelHeavy
    BlockStats { block_type: BlockType::WheelHeavy,     display_name: "Heavy Wheel",      mass: 40.0, max_health: 200.0, deform_threshold: 0.9, armor: 0.5, special: SpecialAbility::None,             color: [0.3,  0.3,  0.35, 1.0 ] },
    // 27 WheelBouncy
    BlockStats { block_type: BlockType::WheelBouncy,    display_name: "Bouncy Wheel",     mass: 10.0, max_health: 80.0,  deform_threshold: 0.9, armor: 0.1, special: SpecialAbility::ProjectileBounce, color: [0.2,  0.8,  0.2,  1.0 ] },
];
