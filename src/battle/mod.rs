// battle/mod.rs  (Bevy 0.15)
pub mod damage;
pub mod manager;
pub mod sports;

use bevy::prelude::*;
use manager::*;
use damage::{process_splash_damage, react_to_damage, SplashDamageRequest, DamageEvent};

pub struct BattlePlugin;
impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EpisodeState>()
           .insert_resource(EpisodeInfo::default())
           .add_event::<SplashDamageRequest>()
           .add_event::<DamageEvent>()
           .add_systems(Update, (
               episode_state_machine,
               car_lifecycle_system,
               fragment_lifetime_system,
               sequential_build_system,
           ))
           .add_systems(Update, (
               activate_cars_in_battle,
               manager::player_control_system,
               process_splash_damage,
               react_to_damage.after(process_splash_damage),
               damage::wheel_contact_damage_system,
               damage::ram_damage_system,
               damage::block_crumple_visuals_system,
               platform_fitness_system,
               sports::football_system,
               sports::discus_system,
           ).run_if(in_state(EpisodeState::Battle)))
           .add_systems(OnEnter(EpisodeState::Build),  on_enter_build)
           .add_systems(OnEnter(EpisodeState::Battle), on_enter_battle)
           .add_systems(OnEnter(EpisodeState::Reset),  on_enter_reset);
    }
}
