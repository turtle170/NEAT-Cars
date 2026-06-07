// ui/mod.rs

pub mod hud;
pub mod builder;

use bevy::prelude::*;
use hud::{setup_hud, update_hud};
use builder::{builder_setup, builder_update};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_hud, builder_setup))
           .add_systems(Update, (update_hud, hud::handle_build_button, hud::handle_speed_button, builder_update));
    }
}
