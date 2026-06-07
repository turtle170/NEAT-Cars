// voxel/deformer.rs — Mesh vertex deformation on damage  (Bevy 0.15)

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;

use super::block::SpecialAbility;

#[derive(Event)]
pub struct DeformBlockEvent {
    pub cell_entity:   Entity,
    pub deform_amount: f32,
    pub seed:          f32,
    pub resist:        bool,
}

pub fn deform_system(
    mut events:  EventReader<DeformBlockEvent>,
    handle_q:    Query<&Handle<Mesh>>,
    mut meshes:  ResMut<Assets<Mesh>>,
) {
    for ev in events.read() {
        let Ok(handle) = handle_q.get(ev.cell_entity) else { continue };
        let Some(mesh) = meshes.get_mut(handle) else { continue };
        let deform = if ev.resist { ev.deform_amount * 0.2 } else { ev.deform_amount };
        deform_mesh(mesh, deform, ev.seed);
    }
}

fn deform_mesh(mesh: &mut Mesh, deform_amount: f32, seed: f32) {
    const MAX_DISP: f32 = 0.08;
    const NOISE_SC: f32 = 3.0;

    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else { return };

    for pos in positions.iter_mut() {
        let [px, py, pz] = *pos;
        let nx = perlin(px*NOISE_SC+seed,       py*NOISE_SC+seed+1.) * 2. - 1.;
        let ny = perlin(py*NOISE_SC+seed+2.,     pz*NOISE_SC+seed)   * 2. - 1.;
        let nz = perlin(pz*NOISE_SC+seed+3.,     px*NOISE_SC+seed+5.)* 2. - 1.;
        let scale = MAX_DISP * deform_amount;
        pos[0] += nx * scale;
        pos[1] += ny * scale;
        pos[2] += nz * scale;
    }
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
}

fn perlin(x: f32, y: f32) -> f32 {
    ((x*127.1+y*311.7).sin()*43758.5453).fract().abs()
}

pub fn trigger_deform_from_damage(
    mut damage_events: EventReader<crate::battle::damage::DamageEvent>,
    mut deform_events: EventWriter<DeformBlockEvent>,
) {
    for ev in damage_events.read() {
        let Some(entity) = ev.cell_entity else { continue };
        let seed = ev.target_cell_x as f32 * 7.3
                 + ev.target_cell_y as f32 * 13.1
                 + ev.target_cell_z as f32 * 19.7;
        deform_events.send(DeformBlockEvent {
            cell_entity:   entity,
            deform_amount: (ev.damage / 100.0).min(1.0),
            seed,
            resist: ev.special == SpecialAbility::ResistDeform,
        });
    }
}
