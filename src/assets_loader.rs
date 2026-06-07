// assets_loader/mod.rs — load OBJ weapon meshes + PNG block textures at startup

use bevy::prelude::*;
use std::collections::HashMap;

use crate::voxel::block::BlockType;
use crate::weapons::WeaponType;

// ─────────────────────────────────────────────────────────────────────────────
// Resources
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct BlockTextures {
    pub map: HashMap<u8, Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct WeaponMeshes {
    pub map: HashMap<u8, Handle<Mesh>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Startup system: queue all asset loads
// ─────────────────────────────────────────────────────────────────────────────

pub fn load_assets(
    mut commands:   Commands,
    asset_server:   Res<AssetServer>,
) {
    // ── Block textures ─────────────────────────────────────────────────────
    let texture_files: &[(u8, &str)] = &[
        (BlockType::Steel          as u8, "textures/blocks/steel.jpg"),
        (BlockType::Titanium       as u8, "textures/blocks/titanium.jpg"),
        (BlockType::Wood           as u8, "textures/blocks/wood.jpg"),
        (BlockType::Explosive      as u8, "textures/blocks/explosive.jpg"),
        (BlockType::Shield         as u8, "textures/blocks/shield.jpg"),
        (BlockType::Rubber         as u8, "textures/blocks/rubber.jpg"),
        (BlockType::CarbonFiber    as u8, "textures/blocks/carbon_fiber.jpg"),
        (BlockType::Glass          as u8, "textures/blocks/glass.jpg"),
        (BlockType::Lead           as u8, "textures/blocks/lead.jpg"),
        (BlockType::Ceramic        as u8, "textures/blocks/ceramic.jpg"),
        (BlockType::ReactiveArmor  as u8, "textures/blocks/reactive_armor.jpg"),
        (BlockType::NanoComposite  as u8, "textures/blocks/nano_composite.jpg"),
        (BlockType::Magnet         as u8, "textures/blocks/magnet.jpg"),
        (BlockType::Spikes         as u8, "textures/blocks/spikes.jpg"),
        (BlockType::Foam           as u8, "textures/blocks/foam.jpg"),
        (BlockType::Gold           as u8, "textures/blocks/gold.jpg"),
        // Remaining 6 fall back to procedural color in block.rs
        (BlockType::Ice            as u8, "textures/blocks/ice.png"),
        (BlockType::Concrete       as u8, "textures/blocks/concrete.png"),
        (BlockType::Mesh           as u8, "textures/blocks/mesh.png"),
        (BlockType::PlasmaCore     as u8, "textures/blocks/plasma_core.png"),
        (BlockType::DepletedUranium as u8, "textures/blocks/depleted_uranium.png"),
        (BlockType::Honeycomb      as u8, "textures/blocks/honeycomb.png"),
    ];

    let mut block_textures = BlockTextures::default();
    for (id, path) in texture_files {
        let handle: Handle<Image> = asset_server.load(*path);
        block_textures.map.insert(*id, handle);
    }
    commands.insert_resource(block_textures);

    // ── Weapon OBJ meshes ─────────────────────────────────────────────────
    let weapon_files: &[(u8, &str)] = &[
        (WeaponType::Rpg              as u8, "meshes/weapons/rpg.obj"),
        (WeaponType::Minigun          as u8, "meshes/weapons/minigun.obj"),
        (WeaponType::SelfAimingTurret as u8, "meshes/weapons/self_aiming_turret.obj"),
        (WeaponType::Flamethrower     as u8, "meshes/weapons/flamethrower.obj"),
        (WeaponType::EmpCannon        as u8, "meshes/weapons/emp_cannon.obj"),
        (WeaponType::LaserBeam        as u8, "meshes/weapons/laser_beam.obj"),
        (WeaponType::GrenadeLauncher  as u8, "meshes/weapons/grenade_launcher.obj"),
        (WeaponType::Shotgun          as u8, "meshes/weapons/shotgun.obj"),
        (WeaponType::SniperRailGun    as u8, "meshes/weapons/sniper_railgun.obj"),
        (WeaponType::HomingMissile    as u8, "meshes/weapons/homing_missile.obj"),
        (WeaponType::MineLayer        as u8, "meshes/weapons/mine_layer.obj"),
        (WeaponType::TeslaCoil        as u8, "meshes/weapons/tesla_coil.obj"),
        (WeaponType::PlasmaCannon     as u8, "meshes/weapons/plasma_cannon.obj"),
        (WeaponType::Mortar           as u8, "meshes/weapons/mortar.obj"),
        (WeaponType::StickyBomb       as u8, "meshes/weapons/sticky_bomb.obj"),
        (WeaponType::Drill            as u8, "meshes/weapons/drill.obj"),
    ];

    let mut weapon_meshes = WeaponMeshes::default();
    for (id, path) in weapon_files {
        let handle: Handle<Mesh> = asset_server.load(*path);
        weapon_meshes.map.insert(*id, handle);
    }
    commands.insert_resource(weapon_meshes);
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin
// ─────────────────────────────────────────────────────────────────────────────

pub struct AssetsPlugin;
impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlockTextures>()
           .init_resource::<WeaponMeshes>()
           .add_systems(Startup, load_assets);
    }
}
