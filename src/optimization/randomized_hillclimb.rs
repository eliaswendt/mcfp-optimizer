use std::{iter::Map, time::Instant};

use petgraph::graph::DiGraph;
use rand::Rng;

use crate::model::{
    group::Group,
    path::Path,
    graph_weigth::{TimetableEdge, TimetableNode},
};

use super::SelectionState;

/// perform a single Hill Climbing Step
pub fn randomized_hillclimb<'a>(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &'a Vec<Group>,
    n_restarts: u64,       // number of "parallel" hill-climb searches
    max_n_iterations: u64, // number of iterations to improve result
) -> SelectionState<'a> {
    println!(
        "randomized_hillclimb(n_runs={}, n_iterations={})",
        n_restarts, max_n_iterations
    );

    // println!("groups_paths={:?}", groups_paths);

    // from each parallel state save the resulting local maximum as (cost, state)
    let mut local_minima: Vec<(u64, SelectionState)> = Vec::with_capacity(n_restarts as usize);

    for run in 0..n_restarts {
        // choose random configuration as initial state
        let initial_state = SelectionState::generate_random_state(groups);
        let mut local_minimum = (initial_state.get_cost(graph), initial_state);

        print!(
            "[restart={}/{}]: initial_cost={} ",
            run + 1,
            n_restarts,
            local_minimum.0
        );

        for j in 0..max_n_iterations {
            // search local maximum from this initial configuration

            let neighbors = local_minimum.1.generate_direct_neighbors();

            // attach each neighbor state with a cost value
            let mut neighbors_with_costs: Vec<(u64, SelectionState)> = neighbors
                .into_iter()
                .map(|s| (s.get_cost(graph), s))
                .collect();


            // sort neighbors by cost (lowest first)
            neighbors_with_costs.sort_unstable_by_key(|(cost, _)| *cost);

            if neighbors_with_costs.len() == 0 || neighbors_with_costs[0].0 >= local_minimum.0 {
                // no neighbors found OR best neighbor has higher cost than current local maximum

                println!(
                    "reached local minimum {} in {}/{} iterations",
                    local_minimum.0,
                    j + 1,
                    max_n_iterations
                );

                // as we won't find any better solution -> early exit loop
                break;
            }

            // println!("found new local maximum neighbor cost={}", neighbors[0].0);

            // set as new local maximum
            neighbors_with_costs.reverse();
            local_minimum = neighbors_with_costs.pop().unwrap();
        }

        local_minima.push(local_minimum);
    }

    local_minima.sort_unstable_by_key(|(cost, _)| *cost);
    println!("lowest local minimum: {:?}", local_minima[0].0);


    // move miminum to end of vec and pop this element
    local_minima.reverse();
    return local_minima.pop().unwrap().1

    // // stores the index of the currently selected path in each group
    // let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());
}
