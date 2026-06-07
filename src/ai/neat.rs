// ai/neat.rs — NEAT Evolution Manager for Bevy
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use std::sync::{Mutex, atomic::{AtomicUsize, Ordering}};
use rayon::prelude::*;

use super::neat_core::Genome;

pub const OBS_DIM: usize = 94;
pub const ACTION_DIM: usize = 30;
pub const POPULATION_SIZE: usize = 20;

#[derive(Resource)]
pub struct NeatManager {
    pub population: Vec<Genome>,
    pub generation: usize,
    pub global_innovation: AtomicUsize,
    // Innovation history: (in_node, out_node) -> innovation_number
    pub innovation_history: Mutex<HashMap<(usize, usize), usize>>,
    pub global_node_id: AtomicUsize,
    pub fitnesses: Vec<f32>,
}

impl Default for NeatManager {
    fn default() -> Self {
        let mut pop = Vec::new();
        // Create initial population
        for _ in 0..POPULATION_SIZE {
            pop.push(Genome::new(OBS_DIM, ACTION_DIM));
        }
        
        let initial_innovations = OBS_DIM * ACTION_DIM;
        Self {
            population: pop,
            generation: 1,
            global_innovation: AtomicUsize::new(initial_innovations),
            innovation_history: Mutex::new(HashMap::new()),
            global_node_id: AtomicUsize::new(OBS_DIM + ACTION_DIM),
            fitnesses: vec![0.0; POPULATION_SIZE],
        }
    }
}

impl NeatManager {
    pub fn get_innovation(&self, in_node: usize, out_node: usize) -> usize {
        let mut history = self.innovation_history.lock().unwrap();
        if let Some(&innov) = history.get(&(in_node, out_node)) {
            innov
        } else {
            let innov = self.global_innovation.fetch_add(1, Ordering::SeqCst);
            history.insert((in_node, out_node), innov);
            innov
        }
    }

    pub fn evolve(&mut self) {
        // Very basic NEAT evolution: Elitism + Crossover + Mutation
        // Sort by fitness descending
        let pop_cloned = self.population.clone();
        let mut indexed: Vec<(usize, Genome)> = pop_cloned.into_iter().enumerate().collect();
        indexed.sort_by(|a, b| b.1.fitness.partial_cmp(&a.1.fitness).unwrap_or(std::cmp::Ordering::Equal));

        let mut next_gen = Vec::new();

        // Elitism: Keep top 10%
        let elite_count = (POPULATION_SIZE as f32 * 0.1) as usize;
        for i in 0..elite_count {
            next_gen.push(indexed[i].1.clone());
        }

        let mut next_gen_par: Vec<Genome> = (next_gen.len()..POPULATION_SIZE).into_par_iter().map(|_| {
            let mut rng = rand::thread_rng();
            let p1_idx = &indexed[rng.gen_range(0..(POPULATION_SIZE / 2))].1;
            let p2_idx = &indexed[rng.gen_range(0..(POPULATION_SIZE / 2))].1;
            
            let mut child = Genome::crossover(p1_idx, p2_idx);
            child.mutate_weights(0.8, 0.5);

            let g_nodes = self.global_node_id.load(Ordering::SeqCst);
            if rng.gen::<f32>() < 0.05 {
                let in_n = rng.gen_range(0..g_nodes);
                let out_n = rng.gen_range(OBS_DIM..g_nodes);
                if in_n != out_n {
                    let innov = self.get_innovation(in_n, out_n);
                    child.add_connection(in_n, out_n, innov);
                }
            }

            if rng.gen::<f32>() < 0.03 && !child.connections.is_empty() {
                let c_idx = rng.gen_range(0..child.connections.len());
                let in_n = child.connections[c_idx].in_node;
                let out_n = child.connections[c_idx].out_node;
                
                let new_node = self.global_node_id.fetch_add(1, Ordering::SeqCst);

                let innov1 = self.get_innovation(in_n, new_node);
                let innov2 = self.get_innovation(new_node, out_n);

                child.add_node(c_idx, new_node, innov1, innov2);
            }

            child
        }).collect();

        next_gen.append(&mut next_gen_par);

        self.population = next_gen;
        self.generation += 1;
    }
}

pub struct NeatPlugin;

impl Plugin for NeatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NeatManager::default());
    }
}
