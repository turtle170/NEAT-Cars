// physics/mod.rs

pub mod car_controller;

use bevy::prelude::*;
use car_controller::car_drive_system;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, car_drive_system);
    }
}
