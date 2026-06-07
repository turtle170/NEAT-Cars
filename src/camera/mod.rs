// camera/mod.rs

pub mod freecam;

use bevy::prelude::*;
use freecam::{FreeCamState, freecam_system, spawn_camera};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FreeCamState::default())
           .add_systems(Startup, spawn_camera)
           .add_systems(Update, freecam_system);
    }
}
