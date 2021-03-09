use std::thread::current;

use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{graph_weigth::{TimetableEdge, TimetableNode}, group::Group, path::Path};

fn time_to_temperature(time: f64) -> f64 {
    100.0 / time.powf(2.0)
}

pub fn simulated_annealing<'a>(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>) -> SelectionState<'a> {

    let mut rng = rand::thread_rng();

    let initial_state = SelectionState::generate_random_state(groups);

    let mut current = (initial_state.get_cost(graph), initial_state);
    let mut time = 1;

    loop {
        let temperature = time_to_temperature(time as f64);
        
        print!("[time={}]: current_state_cost={}, temperature={}, ", time, current.0, temperature);
        
        // actually exactly zero, but difficult with float
        if temperature < 0.1 {
            println!("-> return");
            return current.1;
        }

        let neighbor_states = current.1.generate_direct_neighbors();
        // attach each neighbor state with a cost value
        let mut neighbors_with_costs: Vec<(u64, SelectionState)> = neighbor_states
            .into_iter()
            .map(|s| (s.get_cost(graph), s))
            .collect();


        // sort neighbors by cost (lowest first)
        neighbors_with_costs.sort_unstable_by_key(|(cost, _)| *cost);

        // select random next state
        // let next_state = &neighbor_states[rng.gen::<usize>() % neighbor_states.len()];
        let next = &neighbors_with_costs[0];

        // print!("next_state={:?}, ", next_state.groups_paths_selection);

        // if next_state is better than current_state -> delta positive
        // if next_state is worse than current_state -> delta negative
        let delta_cost = current.0 as i64 - next.0 as i64;

        print!("delta_cost={}, ", delta_cost);

        if delta_cost > 0 {
            current = next.clone();
            println!("current_state = next_state");
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