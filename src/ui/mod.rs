// ui/mod.rs

pub mod hud;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use hud::{setup_hud, update_hud, speed_slider_system};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
           .add_systems(Startup, setup_hud)
           .add_systems(Update, (update_hud, speed_slider_system));
    }
}
