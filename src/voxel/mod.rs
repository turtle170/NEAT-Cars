// voxel/mod.rs  (Bevy 0.15)

pub mod block;
pub mod builder;
pub mod deformer;
pub mod grid;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use deformer::{DeformBlockEvent, deform_system, trigger_deform_from_damage};
use grid::VoxelGrid;

pub struct VoxelPlugin;
impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DeformBlockEvent>()
           .add_systems(Update, trigger_deform_from_damage)
           .add_systems(Update, deform_system.after(trigger_deform_from_damage))
           .add_systems(Update, sync_grid_colliders);
    }
}

pub fn sync_grid_colliders(
    mut commands: Commands,
    mut grid_q: Query<(Entity, &mut VoxelGrid, &GlobalTransform), Changed<VoxelGrid>>,
    mut wheels_q: Query<&mut crate::physics::car_controller::CarWheels>,
    attached_wheels_q: Query<&crate::physics::car_controller::AttachedWheel>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    for (grid_ent, mut grid, gtf) in grid_q.iter_mut() {
        // 1. Detach loose parts (flood-fill check)
        let detached = grid.detach_loose_parts();
        for (local_pos, _block_type, cell_ent) in detached {
            if let Some(ent) = cell_ent { commands.entity(ent).despawn_recursive(); }
            
            // Spawn detached blocks as physical debris
            use crate::voxel::grid::BLOCK_SIZE;
            use rand::Rng;
            let mut rng = rand::thread_rng();
            
            let (frag_mesh, frag_mat) = cache.get_or_insert_with(|| {
                (
                    meshes.add(Cuboid::new(BLOCK_SIZE, BLOCK_SIZE, BLOCK_SIZE)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.35, 0.35, 0.35),
                        ..default()
                    })
                )
            });
            
            let world_pos = gtf.transform_point(local_pos);
            commands.spawn((
                PbrBundle {
                    mesh: frag_mesh.clone(),
                    material: frag_mat.clone(),
                    transform: Transform::from_translation(world_pos),
                    ..default()
                },
                RigidBody::Dynamic,
                Collider::cuboid(BLOCK_SIZE*0.3, BLOCK_SIZE*0.3, BLOCK_SIZE*0.3), // Slightly smaller to prevent overlap overlap explosions
                ColliderMassProperties::Mass(1.0), // Explicit mass to prevent extreme impulse velocities
                ExternalImpulse {
                    impulse: Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(0.5..2.0), rng.gen_range(-1.0..1.0)).normalize_or_zero() * 2.5,
                    ..default()
                },
                Velocity::default(),
                crate::battle::manager::FragmentMarker { lifetime: 6.0 },
                crate::physics::car_controller::Owner(grid_ent),
            ));
        }
        
        // 2. Build and assign the exact compound collider
        let new_collider = grid.build_compound_collider();
        commands.entity(grid_ent).insert(new_collider);
        
        // 3. Update wheels (detach if block is destroyed)
        if let Ok(mut wheels) = wheels_q.get_mut(grid_ent) {
            wheels.wheels.retain(|w_ent| {
                if let Ok(attached) = attached_wheels_q.get(*w_ent) {
                    let [x, y, z] = attached.grid_pos;
                    if grid.get(x, y, z).is_none() {
                        // The wheel block was destroyed/detached. Despawn the wheel.
                        commands.entity(*w_ent).despawn_recursive();
                        return false;
                    }
                }
                true
            });
        }
    }
}
