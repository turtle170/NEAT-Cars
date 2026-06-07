// voxel/grid.rs — 10×10×10 VoxelGrid Bevy Component  (Bevy 0.15)

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::block::{BlockStats, BlockType, SpecialAbility};
use crate::battle::damage::DamageEvent;

pub const GRID_SIZE: usize = 10;
pub const BLOCK_SIZE: f32  = 0.4;

#[derive(Debug, Clone)]
pub struct VoxelCell {
    pub block_type:         BlockType,
    pub current_health:     f32,
    pub max_health:         f32,
    pub absorbed_first_hit: bool,
    pub repair_accum:       f32,
    pub entity:             Option<Entity>,
}

impl VoxelCell {
    pub fn new(t: BlockType) -> Self {
        let s = BlockStats::get(t);
        Self { block_type: t, current_health: s.max_health, max_health: s.max_health,
               absorbed_first_hit: false, repair_accum: 0., entity: None }
    }
    pub fn health_frac(&self) -> f32 { if self.max_health > 0. { self.current_health / self.max_health } else { 0. } }
    pub fn is_alive(&self)   -> bool { self.current_health > 0. }
}

#[derive(Component, Debug, Clone)]
pub struct VoxelGrid {
    cells:          [[[Option<VoxelCell>; GRID_SIZE]; GRID_SIZE]; GRID_SIZE],
    pub block_count: usize,
    pub total_mass:  f32,
    pub local_com:   Vec3,
}

impl Default for VoxelGrid {
    fn default() -> Self {
        Self { cells: core::array::from_fn(|_| core::array::from_fn(|_| core::array::from_fn(|_| None))),
               block_count: 0, total_mass: 0., local_com: Vec3::ZERO }
    }
}

impl VoxelGrid {
    pub fn in_bounds(x: i32, y: i32, z: i32) -> bool {
        let s = GRID_SIZE as i32;
        x >= 0 && x < s && y >= 0 && y < s && z >= 0 && z < s
    }

    pub fn cell_local_pos(x: usize, y: usize, z: usize) -> Vec3 {
        let h = GRID_SIZE as f32 * 0.5;
        Vec3::new((x as f32-h+0.5)*BLOCK_SIZE, (y as f32-h+0.5)*BLOCK_SIZE, (z as f32-h+0.5)*BLOCK_SIZE)
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<&VoxelCell> { self.cells[x][y][z].as_ref() }
    pub fn get_mut(&mut self, x: usize, y: usize, z: usize) -> Option<&mut VoxelCell> { self.cells[x][y][z].as_mut() }

    pub fn place(&mut self, x: usize, y: usize, z: usize, t: BlockType) {
        if t == BlockType::Empty { self.remove(x, y, z); return; }
        let was_none = self.cells[x][y][z].is_none();
        self.cells[x][y][z] = Some(VoxelCell::new(t));
        if was_none { self.block_count += 1; }
        self.recalculate_mass();
    }

    pub fn remove(&mut self, x: usize, y: usize, z: usize) -> Option<VoxelCell> {
        let old = self.cells[x][y][z].take();
        if old.is_some() { self.block_count = self.block_count.saturating_sub(1); }
        self.recalculate_mass();
        old
    }

    pub fn recalculate_mass(&mut self) {
        let mut mass = 0.0_f32; let mut com = Vec3::ZERO; let mut count = 0usize;
        for x in 0..GRID_SIZE { for y in 0..GRID_SIZE { for z in 0..GRID_SIZE {
            if let Some(cell) = &self.cells[x][y][z] {
                let m = BlockStats::get(cell.block_type).mass;
                let p = Self::cell_local_pos(x, y, z);
                mass += m; com += p * m; count += 1;
            }
        }}}
        self.total_mass = mass;
        self.local_com  = if mass > 0. { com / mass } else { Vec3::ZERO };
        self.block_count = count;
    }

    pub fn apply_damage_local(
        &mut self, local_pos: Vec3, base_damage: f32, radius: f32,
        attacker: Entity, events: &mut EventWriter<DamageEvent>,
    ) {
        for x in 0..GRID_SIZE { for y in 0..GRID_SIZE { for z in 0..GRID_SIZE {
            let cell_pos = Self::cell_local_pos(x, y, z);
            let dist = (local_pos - cell_pos).length();
            if dist > radius { continue; }

            let cell = match self.cells[x][y][z].as_mut() {
                Some(c) if c.is_alive() => c,
                _ => continue,
            };

            let stats = BlockStats::get(cell.block_type);

            if stats.special == SpecialAbility::AbsorbFirstHit && !cell.absorbed_first_hit {
                cell.absorbed_first_hit = true;
                continue;
            }
            if stats.special == SpecialAbility::ProjectileBounce {
                use rand::Rng;
                if rand::thread_rng().gen::<f32>() < 0.4 { continue; }
            }

            let falloff = 1.0 - (dist / radius).min(1.0);
            let mut dmg = base_damage * falloff / (1. + stats.armor);

            if stats.special == SpecialAbility::DamageReduction { dmg *= 0.5; }
            if stats.special == SpecialAbility::Brittle && !cell.absorbed_first_hit {
                dmg *= 3.0; cell.absorbed_first_hit = true;
            }

            cell.current_health -= dmg;

            events.send(DamageEvent {
                target_cell_x: x as u8, target_cell_y: y as u8, target_cell_z: z as u8,
                damage: dmg, attacker,
                died: cell.current_health <= 0.0,
                cell_entity: cell.entity,
                special: stats.special,
            });

            if cell.current_health <= 0.0 {
                self.cells[x][y][z] = None;
                self.block_count = self.block_count.saturating_sub(1);
                self.recalculate_mass();
            }
        }}}
    }

    pub fn iter_cells(&self) -> impl Iterator<Item = (usize, usize, usize, &VoxelCell)> {
        self.cells.iter().enumerate().flat_map(|(x, yz)| {
            yz.iter().enumerate().flat_map(move |(y, zz)| {
                zz.iter().enumerate().filter_map(move |(z, c)| {
                    c.as_ref().map(move |cell| (x, y, z, cell))
                })
            })
        })
    }

    pub fn build_compound_collider(&self) -> Collider {
        let mut parts = Vec::new();
        let half = BLOCK_SIZE * 0.5;
        let shape = Collider::cuboid(half, half, half);
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                for z in 0..GRID_SIZE {
                    if let Some(cell) = &self.cells[x][y][z] {
                        if !cell.block_type.is_wheel() {
                            let pos = Self::cell_local_pos(x, y, z);
                            parts.push((pos, Quat::IDENTITY, shape.clone()));
                        }
                    }
                }
            }
        }
        if parts.is_empty() {
            Collider::cuboid(0.1, 0.1, 0.1) // Fallback to prevent crash if car is completely destroyed
        } else {
            Collider::compound(parts)
        }
    }

    pub fn detach_loose_parts(&mut self) -> Vec<(Vec3, BlockType, Option<Entity>)> {
        let mut visited = [[[false; GRID_SIZE]; GRID_SIZE]; GRID_SIZE];
        let mut queue = std::collections::VecDeque::new();
        
        let mut best_dist = f32::MAX;
        let mut best_start = None;
        let center_idx = (GRID_SIZE / 2) as f32;
        
        // Find core block closest to the center
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                for z in 0..GRID_SIZE {
                    if self.cells[x][y][z].is_some() {
                        let dist = (x as f32 - center_idx).powi(2) +
                                   (y as f32 - center_idx).powi(2) +
                                   (z as f32 - center_idx).powi(2);
                        if dist < best_dist {
                            best_dist = dist;
                            best_start = Some((x,y,z));
                        }
                    }
                }
            }
        }
        
        // Flood fill from core
        if let Some((sx,sy,sz)) = best_start {
            queue.push_back((sx,sy,sz));
            visited[sx][sy][sz] = true;
            
            let dirs = [(1,0,0), (-1,0,0), (0,1,0), (0,-1,0), (0,0,1), (0,0,-1)];
            while let Some((cx,cy,cz)) = queue.pop_front() {
                for (dx,dy,dz) in dirs.iter() {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    let nz = cz as i32 + dz;
                    if Self::in_bounds(nx, ny, nz) {
                        let (ux, uy, uz) = (nx as usize, ny as usize, nz as usize);
                        if !visited[ux][uy][uz] && self.cells[ux][uy][uz].is_some() {
                            visited[ux][uy][uz] = true;
                            queue.push_back((ux,uy,uz));
                        }
                    }
                }
            }
        }
        
        let mut detached = Vec::new();
        for x in 0..GRID_SIZE {
            for y in 0..GRID_SIZE {
                for z in 0..GRID_SIZE {
                    if self.cells[x][y][z].is_some() && !visited[x][y][z] {
                        let cell = self.remove(x, y, z).unwrap();
                        let pos = Self::cell_local_pos(x, y, z);
                        detached.push((pos, cell.block_type, cell.entity));
                    }
                }
            }
        }
        detached
    }
}
