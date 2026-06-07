pub mod agent;
pub mod neat_core;
pub mod neat;

use bevy::prelude::*;
use neat::NeatPlugin;
use agent::{ai_step_system, agent_step_reward_system};

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NeatPlugin);
        app.add_systems(FixedUpdate, (ai_step_system, agent_step_reward_system)
            .run_if(in_state(crate::battle::manager::EpisodeState::Battle)));
    }
}
