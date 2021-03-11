use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::Path};

/// maps time to temperature value
fn time_to_temperature(time: f64) -> f64 {
    // 100.0 / time.powf(2.0)
    100.0 / (time as f64)
}

pub fn simulated_annealing<'a>(graph: &'a mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>) -> SelectionState<'a> {

    let mut rng = rand::thread_rng();

    let mut current = SelectionState::generate_random_state(graph, groups);
    let mut time = 1;

    loop {
        let temperature = time_to_temperature(time as f64);
        
        print!("[time={}]: current_state_cost={}, temperature={}, ", time, current.cost, temperature);
        
        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            println!("-> return");
            return current;
        }

        // select random next state
        // let next_state = &neighbor_states[rng.gen::<usize>() % neighbor_states.len()];
        let next = current
            .generate_direct_neighbors(graph)
            .into_iter()
            .min_by_key(|s| s.cost)
            .unwrap();

  
        // print!("next_state={:?}, ", next_state.groups_paths_selection);

        // if next_state is better than current_state -> delta positive
        // if next_state is worse than current_state -> delta negative
        let delta_cost = current.cost as i64 - next.cost as i64;

        print!("delta_cost={}, ", delta_cost);

        if delta_cost > 0 {
            current = next.clone();
            println!("replacing current state");
        } else {
            let probability = (delta_cost as f64 / temperature as f64).exp();
            let random = rng.gen_range(0.0..1.0);

            println!("probability={}, random={}", probability, random);

            if random < probability {
                current = next.clone();
            }
        }

        time += 1;
    }
}