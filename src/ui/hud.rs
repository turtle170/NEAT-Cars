// ui/hud.rs

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::battle::manager::{EpisodeInfo, EpisodeState};
use crate::voxel::grid::VoxelGrid;
use crate::ai::agent::CarAgent;

#[derive(Component)] pub struct PhaseLabel;
#[derive(Component)] pub struct TimerLabel;
#[derive(Component)] pub struct EpisodeLabel;
#[derive(Component)] pub struct AgentCountLabel;
#[derive(Component)] pub struct AgentStatsLabel { pub agent_id: usize }
#[derive(Component)] pub struct CarTextLabel;

#[derive(Resource)]
pub struct SimSpeedState {
    pub speed: f32,
}
impl Default for SimSpeedState {
    fn default() -> Self { Self { speed: 1.0 } }
}

const MAX_BARS: usize = 15;

pub fn setup_hud(mut commands: Commands) {
    commands.insert_resource(SimSpeedState::default());
    
    // Full-screen root
    commands.spawn(NodeBundle {
        style: Style {
            width:          Val::Percent(100.),
            height:         Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items:    AlignItems::FlexStart,
            padding:        UiRect::all(Val::Px(12.)),
            row_gap:        Val::Px(4.),
            ..default()
        },
        ..default()
    }).with_children(|root| {
        // Header row
        root.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(20.), align_items: AlignItems::Center, ..default() },
            ..default()
        }).with_children(|row| {
            spawn_text(row, "Phase: Build",  PhaseLabel);
            spawn_text(row, "Time: --",      TimerLabel);
            spawn_text(row, "Episode: 0",    EpisodeLabel);
            spawn_text(row, "Cars: 0",       AgentCountLabel);
        });

        // Stats Panel (absolute right)
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right:  Val::Px(16.),
                top:    Val::Px(16.),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.),
                width:  Val::Px(240.),
                ..default()
            },
            ..default()
        }).with_children(|col| {
            for i in 0..MAX_BARS {
                col.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Percent(100.),
                            padding: UiRect::all(Val::Px(4.)),
                            ..default()
                        },
                        background_color: Color::NONE.into(),
                        ..default()
                    },
                    AgentStatsLabel { agent_id: i },
                )).with_children(|btn| {
                    btn.spawn((
                        TextBundle::from_section(
                            format!("Car {}: Rwd: 0  Fit: 0", i),
                            TextStyle { font_size: 13., color: Color::WHITE, ..default() },
                        ),
                        CarTextLabel
                    ));
                });
            }
        });
    });
}

fn spawn_text<M: Component>(parent: &mut ChildBuilder, text: &str, marker: M) {
    parent.spawn((
        TextBundle::from_section(
            text,
            TextStyle { font_size: 16., color: Color::WHITE, ..default() },
        ).with_style(Style {
            padding: UiRect::all(Val::Px(4.)),
            ..default()
        }),
        marker,
    ));
}

pub fn update_hud(
    info:        Res<EpisodeInfo>,
    state:       Res<State<EpisodeState>>,
    mut phase_q: Query<&mut Text, (With<PhaseLabel>, Without<TimerLabel>, Without<EpisodeLabel>, Without<AgentCountLabel>, Without<CarTextLabel>)>,
    mut timer_q: Query<&mut Text, (With<TimerLabel>, Without<PhaseLabel>, Without<EpisodeLabel>, Without<AgentCountLabel>, Without<CarTextLabel>)>,
    mut ep_q:    Query<&mut Text, (With<EpisodeLabel>, Without<PhaseLabel>, Without<TimerLabel>, Without<AgentCountLabel>, Without<CarTextLabel>)>,
    mut ac_q:    Query<&mut Text, (With<AgentCountLabel>, Without<PhaseLabel>, Without<TimerLabel>, Without<EpisodeLabel>, Without<CarTextLabel>)>,
    car_q:       Query<(Entity, &VoxelGrid, &CarAgent)>,
    mut btn_q:   Query<(&Interaction, &AgentStatsLabel, &Children)>,
    mut text_q:  Query<&mut Text, With<CarTextLabel>>,
    neat_mgr:    Res<crate::ai::neat::NeatManager>,
    mut freecam: ResMut<crate::camera::freecam::FreeCamState>,
) {
    let phase_str = format!("Phase: {:?}", state.get());
    for mut t in phase_q.iter_mut() { t.sections[0].value = phase_str.clone(); }
    for mut t in timer_q.iter_mut() { t.sections[0].value = format!("Time: {:.0}s", info.phase_timer); }
    for mut t in ep_q.iter_mut()    { t.sections[0].value = format!("Episode: {}", info.episode); }
    for mut t in ac_q.iter_mut()    { t.sections[0].value = format!("Cars: {}",    info.agent_count); }

    let mut cars: Vec<_> = car_q.iter().collect();
    cars.sort_by_key(|(_, _, agent)| agent.agent_id);

    for (interaction, lbl, children) in btn_q.iter_mut() {
        let car_opt = cars.get(lbl.agent_id);
        
        // Handle clicking
        if *interaction == Interaction::Pressed {
            if let Some(&(car_ent, _, _)) = car_opt {
                if freecam.follow == Some(car_ent) {
                    freecam.follow = None; // detach
                } else {
                    freecam.follow = Some(car_ent); // attach
                }
            }
        }

        // Update text
        if let Some(&child) = children.first() {
            if let Ok(mut text) = text_q.get_mut(child) {
                if let Some(&(_, g, agent)) = car_opt {
                    let alive = agent.is_alive(g);
                    let reward = agent.accumulated_reward;
                    let fit = neat_mgr.population.get(agent.agent_id).map(|a| a.fitness).unwrap_or(0.0);
                    text.sections[0].value = format!(
                        "Car {}{}: Rwd: {:.1}  Fit: {:.1}", 
                        agent.agent_id, 
                        if alive { "" } else { " ☠️" },
                        reward,
                        fit
                    );
                    text.sections[0].style.color = if alive { Color::WHITE } else { Color::rgb(0.6, 0.2, 0.2) };
                } else {
                    text.sections[0].value = format!("Car {} - N/A", lbl.agent_id);
                    text.sections[0].style.color = Color::rgb(0.3, 0.3, 0.3);
                }
            }
        }
    }
}

pub fn speed_slider_system(
    mut contexts: EguiContexts,
    mut speed_state: ResMut<SimSpeedState>,
    mut time: ResMut<Time<Virtual>>,
    ep_state: Res<State<EpisodeState>>,
) {
    egui::Window::new("Simulation Speed")
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 50.0))
        .collapsible(false)
        .title_bar(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Speed:");
                let mut speed = speed_state.speed;
                if ui.add(egui::Slider::new(&mut speed, 1.0..=10.0).text("x")).changed() {
                    speed_state.speed = speed;
                    if *ep_state.get() == EpisodeState::Battle {
                        time.set_relative_speed(speed);
                    }
                }
            });
        });
}
