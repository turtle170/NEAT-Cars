// voxel/builder.rs — Genome → Car builder  (Bevy 0.15)

use bevy::prelude::*;
use rand::Rng;
use std::collections::VecDeque;

use super::block::BlockType;
use super::grid::{VoxelGrid, BLOCK_SIZE, GRID_SIZE};
use crate::weapons::WeaponType;

#[derive(Debug, Clone, Copy)]
pub struct WeaponPlacement {
    pub grid_pos:    [usize; 3],
    pub weapon_type: WeaponType,
}

#[derive(Component)]
pub struct CarBuilder {
    pub max_weapons: usize,
}

impl Default for CarBuilder {
    fn default() -> Self { Self { max_weapons: 3 } }
}

pub const GENOME_LEN: usize = GRID_SIZE * GRID_SIZE * GRID_SIZE;

pub fn genome_index(x: usize, y: usize, z: usize) -> usize {
    x + y * GRID_SIZE + z * GRID_SIZE * GRID_SIZE
}

pub fn apply_genome(grid: &mut VoxelGrid, genome: &[u8; GENOME_LEN], _weapons: &[WeaponPlacement]) {
    *grid = VoxelGrid::default();
    for x in 0..GRID_SIZE { for y in 0..GRID_SIZE { for z in 0..GRID_SIZE {
        let t = BlockType::from_u8(genome[genome_index(x, y, z)]);
        if t != BlockType::Empty { grid.place(x, y, z, t); }
    }}}
    enforce_connectivity(grid);
    if grid.block_count < 4 { build_default_chassis(grid); }
}

fn enforce_connectivity(grid: &mut VoxelGrid) {
    let mut visited = [[[false; GRID_SIZE]; GRID_SIZE]; GRID_SIZE];
    let mut queue   = VecDeque::new();
    for x in 0..GRID_SIZE { for z in 0..GRID_SIZE {
        if grid.get(x, 0, z).is_some() { visited[x][0][z] = true; queue.push_back((x, 0usize, z)); }
    }}
    const DIRS: [(i32,i32,i32); 6] = [(1,0,0),(-1,0,0),(0,1,0),(0,-1,0),(0,0,1),(0,0,-1)];
    while let Some((cx, cy, cz)) = queue.pop_front() {
        for (dx,dy,dz) in &DIRS {
            let (nx,ny,nz) = (cx as i32+dx, cy as i32+dy, cz as i32+dz);
            if !VoxelGrid::in_bounds(nx,ny,nz) { continue; }
            let (nx,ny,nz) = (nx as usize, ny as usize, nz as usize);
            if visited[nx][ny][nz] || grid.get(nx,ny,nz).is_none() { continue; }
            visited[nx][ny][nz] = true; queue.push_back((nx,ny,nz));
        }
    }
    for x in 0..GRID_SIZE { for y in 0..GRID_SIZE { for z in 0..GRID_SIZE {
        if let Some(block) = grid.get(x,y,z) {
            if !visited[x][y][z] && !block.block_type.is_wheel() {
                grid.remove(x,y,z);
            }
        }
    }}}
}

fn build_default_chassis(grid: &mut VoxelGrid) {
    *grid = VoxelGrid::default();
    for x in 3..7 { for y in 4..6 { for z in 2..8 { grid.place(x, y, z, BlockType::Steel); }}}
}

pub fn random_genome() -> [u8; GENOME_LEN] {
    let mut rng = rand::thread_rng();
    let mut genome = [0u8; GENOME_LEN];
    for g in genome.iter_mut() {
        *g = if rng.gen::<f32>() < 0.6 { 0 } else { rng.gen_range(1..BlockType::count()) };
    }
    genome
}

pub fn random_weapons(max: usize) -> Vec<WeaponPlacement> {
    let mut rng = rand::thread_rng();
    let count = rng.gen_range(1..=max);
    (0..count).map(|_| WeaponPlacement {
        grid_pos:    [rng.gen_range(2..8), rng.gen_range(5..10), rng.gen_range(2..8)],
        weapon_type: WeaponType::from_u8(rng.gen_range(0..16)),
    }).collect()
}

pub fn spawn_block_visuals(
    commands:  &mut Commands,
    meshes:    &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    grid:      &mut VoxelGrid,
    parent:    Entity,
) {
    let cube_mesh = meshes.add(Cuboid::new(BLOCK_SIZE*0.98, BLOCK_SIZE*0.98, BLOCK_SIZE*0.98));
    for x in 0..GRID_SIZE { for y in 0..GRID_SIZE { for z in 0..GRID_SIZE {
        let cell = match grid.get_mut(x,y,z) { Some(c) => c, None => continue };
        if cell.block_type.is_wheel() { continue; }
        use super::block::BlockStats;
        let stats = BlockStats::get(cell.block_type);
        let [r,g,b,a] = stats.color;
        let mat = materials.add(StandardMaterial {
            base_color: Color::rgba(r,g,b,a),
            emissive:   Color::rgb(r*30.0, g*30.0, b*30.0),
            metallic:   if stats.armor > 0.3 { 0.6 } else { 0.1 },
            perceptual_roughness: 0.5,
            alpha_mode: if a < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..default()
        });
        let local = VoxelGrid::cell_local_pos(x,y,z);
        let id = commands.spawn(PbrBundle {
            mesh: cube_mesh.clone(),
            material: mat,
            transform: Transform::from_translation(local),
            ..default()
        }).set_parent(parent).id();
        cell.entity = Some(id);
    }}}
}
