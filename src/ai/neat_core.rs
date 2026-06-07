// ai/neat_core.rs — Custom NEAT implementation
use rand::Rng;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType { Input, Hidden, Output }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGene {
    pub id: usize,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionGene {
    pub in_node: usize,
    pub out_node: usize,
    pub weight: f32,
    pub enabled: bool,
    pub innovation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub nodes: HashMap<usize, NodeGene>,
    pub connections: Vec<ConnectionGene>,
    pub fitness: f32,
}

impl Genome {
    pub fn new(input_count: usize, output_count: usize) -> Self {
        let mut nodes = HashMap::new();
        let mut connections = Vec::new();
        let mut innov = 0;

        for i in 0..input_count {
            nodes.insert(i, NodeGene { id: i, node_type: NodeType::Input });
        }
        for i in 0..output_count {
            let out_id = input_count + i;
            nodes.insert(out_id, NodeGene { id: out_id, node_type: NodeType::Output });
            
            // Connect all inputs to all outputs initially
            for j in 0..input_count {
                connections.push(ConnectionGene {
                    in_node: j,
                    out_node: out_id,
                    weight: rand::thread_rng().gen_range(-1.0..1.0),
                    enabled: true,
                    innovation: innov,
                });
                innov += 1;
            }
        }

        Self { nodes, connections, fitness: 0.0 }
    }

    pub fn mutate_weights(&mut self, mut_rate: f32, mut_power: f32) {
        let mut rng = rand::thread_rng();
        for c in &mut self.connections {
            if rng.gen::<f32>() < mut_rate {
                if rng.gen::<f32>() < 0.1 { // 10% chance to completely randomize
                    c.weight = rng.gen_range(-2.0..2.0);
                } else {
                    c.weight += rng.gen_range(-mut_power..mut_power);
                }
            }
        }
    }

    pub fn add_connection(&mut self, in_node: usize, out_node: usize, innovation: usize) {
        // Simple check to prevent duplicates
        if self.connections.iter().any(|c| c.in_node == in_node && c.out_node == out_node) {
            return;
        }
        self.connections.push(ConnectionGene {
            in_node,
            out_node,
            weight: rand::thread_rng().gen_range(-1.0..1.0),
            enabled: true,
            innovation,
        });
    }

    pub fn add_node(&mut self, conn_index: usize, new_node_id: usize, innov1: usize, innov2: usize) {
        if conn_index >= self.connections.len() { return; }
        
        // Disable old connection
        self.connections[conn_index].enabled = false;
        let in_n = self.connections[conn_index].in_node;
        let out_n = self.connections[conn_index].out_node;
        let old_weight = self.connections[conn_index].weight;

        self.nodes.insert(new_node_id, NodeGene { id: new_node_id, node_type: NodeType::Hidden });
        
        // In -> New (weight 1.0)
        self.connections.push(ConnectionGene {
            in_node: in_n,
            out_node: new_node_id,
            weight: 1.0,
            enabled: true,
            innovation: innov1,
        });

        // New -> Out (old weight)
        self.connections.push(ConnectionGene {
            in_node: new_node_id,
            out_node: out_n,
            weight: old_weight,
            enabled: true,
            innovation: innov2,
        });
    }

    pub fn crossover(parent1: &Genome, parent2: &Genome) -> Genome {
        let mut rng = rand::thread_rng();
        // Assuming parent1 is fitter
        let mut child = Genome {
            nodes: parent1.nodes.clone(),
            connections: Vec::new(),
            fitness: 0.0,
        };

        let mut p2_conns = HashMap::new();
        for c in &parent2.connections {
            p2_conns.insert(c.innovation, c);
        }

        for c1 in &parent1.connections {
            let mut cg = c1.clone();
            if let Some(c2) = p2_conns.get(&c1.innovation) {
                if rng.gen::<bool>() {
                    cg.weight = c2.weight;
                }
                if !c1.enabled || !c2.enabled {
                    if rng.gen::<f32>() < 0.75 { cg.enabled = false; }
                }
            }
            child.connections.push(cg);
        }
        child
    }

    pub fn distance(g1: &Genome, g2: &Genome, c1: f32, c2: f32, c3: f32) -> f32 {
        let inn1: HashSet<usize> = g1.connections.iter().map(|c| c.innovation).collect();
        let inn2: HashSet<usize> = g2.connections.iter().map(|c| c.innovation).collect();
        
        let max_inn1 = inn1.iter().max().copied().unwrap_or(0);
        let max_inn2 = inn2.iter().max().copied().unwrap_or(0);

        let mut disjoint = 0.0;
        let mut excess = 0.0;
        let mut weight_diff = 0.0;
        let mut matching = 0.0;

        for c in &g1.connections {
            if inn2.contains(&c.innovation) {
                matching += 1.0;
                let c2 = g2.connections.iter().find(|x| x.innovation == c.innovation).unwrap();
                weight_diff += (c.weight - c2.weight).abs();
            } else {
                if c.innovation > max_inn2 { excess += 1.0; } else { disjoint += 1.0; }
            }
        }
        for c in &g2.connections {
            if !inn1.contains(&c.innovation) {
                if c.innovation > max_inn1 { excess += 1.0; } else { disjoint += 1.0; }
            }
        }

        let n = (g1.connections.len().max(g2.connections.len()) as f32).max(1.0);
        if matching > 0.0 { weight_diff /= matching; }
        
        (c1 * excess / n) + (c2 * disjoint / n) + (c3 * weight_diff)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Feedforward Evaluator
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct NetOp {
    in_node: usize,
    out_node: usize,
    weight: f32,
}

pub struct FeedforwardNet {
    node_values: Vec<f32>,
    ops: Vec<NetOp>,
    input_ids: Vec<usize>,
    output_ids: Vec<usize>,
    sorted_hidden_out: Vec<usize>,
}

impl FeedforwardNet {
    pub fn new(genome: &Genome, _input_count: usize, _output_count: usize) -> Self {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut hidden = Vec::new();

        for (&id, n) in &genome.nodes {
            match n.node_type {
                NodeType::Input => inputs.push(id),
                NodeType::Output => outputs.push(id),
                NodeType::Hidden => hidden.push(id),
            }
        }
        inputs.sort();
        outputs.sort();

        let mut sorted_hidden_out = hidden.clone();
        sorted_hidden_out.extend(outputs.clone());

        // Pre-compile ops sorted by target node's topological order
        let mut ops = Vec::new();
        for &id in &sorted_hidden_out {
            for c in genome.connections.iter().filter(|c| c.enabled) {
                if c.out_node == id {
                    ops.push(NetOp {
                        in_node: c.in_node,
                        out_node: c.out_node,
                        weight: c.weight,
                    });
                }
            }
        }

        // Find max node id to size the vector
        let max_id = genome.nodes.keys().max().copied().unwrap_or(0);

        Self {
            node_values: vec![0.0; max_id + 1],
            ops,
            input_ids: inputs,
            output_ids: outputs,
            sorted_hidden_out,
        }
    }

    pub fn activate(&mut self, inputs: &[f32]) -> Vec<f32> {
        // Clear old hidden/output values
        for &id in &self.sorted_hidden_out {
            if id < self.node_values.len() {
                self.node_values[id] = 0.0;
            }
        }

        // Set inputs
        for (i, &id) in self.input_ids.iter().enumerate() {
            if id < self.node_values.len() {
                self.node_values[id] = *inputs.get(i).unwrap_or(&0.0);
            }
        }

        // Evaluate nodes in topological order
        let mut op_idx = 0;
        for &id in &self.sorted_hidden_out {
            let mut sum = 0.0;
            while op_idx < self.ops.len() && self.ops[op_idx].out_node == id {
                let op = &self.ops[op_idx];
                if op.in_node < self.node_values.len() {
                    sum += self.node_values[op.in_node] * op.weight;
                }
                op_idx += 1;
            }
            if id < self.node_values.len() {
                self.node_values[id] = 1.0 / (1.0 + (-sum).exp());
            }
        }

        self.output_ids.iter().map(|&id| {
            if id < self.node_values.len() { self.node_values[id] } else { 0.0 }
        }).collect()
    }
}
