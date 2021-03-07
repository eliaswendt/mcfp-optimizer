use petgraph::graph::DiGraph;
use rand::Rng;

use crate::model::{
    graph::{TimetableEdge, TimetableNode},
    group::Group,
    path::Path,
};

/// formalizing a system state
/// by storing indices of currently selected path for each group
#[derive(Debug)]
struct SelectionState<'a> {
    groups_paths: &'a Vec<Vec<&'a Path>>,
    pub groups_paths_selection: Vec<usize>, // array of indices (specifies selected path for each group)
}

impl<'a> SelectionState<'a> {
    pub fn generate_random_state(groups_paths: &'a Vec<Vec<&Path>>) -> Self {
        let mut rng = rand::thread_rng();

        let mut groups_paths_selection = Vec::with_capacity(groups_paths.len());

        for group_paths in groups_paths.iter() {
            groups_paths_selection.push(rng.gen::<usize>() % group_paths.len());
        }

        Self {
            groups_paths,
            groups_paths_selection,
        }
    }

    /// generates a vec of neighbor states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    pub fn generate_neighbors(&self) -> Vec<(u64, Self)> {
        let mut neighbors = Vec::with_capacity(self.groups_paths.len());

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {
            // create state with index decremented by one
            if self.groups_paths_selection[group_index] != 0 {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] -= 1;

                let selection_state = Self {
                    groups_paths: self.groups_paths,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push((
                    selection_state.get_cost(),
                    selection_state
                ));
            }

            if self.groups_paths_selection[group_index] != self.groups_paths[group_index].len() - 1 {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] += 1;

                let selection_state = Self {
                    groups_paths: self.groups_paths,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push((
                    selection_state.get_cost(),
                    selection_state
                ));
            }
        }

        // sort lowest cost to top
        neighbors.sort_unstable_by_key(|(cost, _)| *cost);
        neighbors
    }

    pub fn get_cost(&self) -> u64 {
        let mut cost_sum = 0;

        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            cost_sum += self.groups_paths[group_index][*path_index].cost;
        }

        cost_sum
    }
}

/// perform a single Hill Climbing Step
pub fn randomized_hillclimb(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
    n_runs: u64, // number of "parallel" hill-climb searches
    n_iterations: u64 // number of iterations to improve result
) {
    println!("randomized_hillclimb(n_runs={}, n_iterations={})", n_runs, n_iterations);


    let groups_paths: Vec<Vec<&Path>> = groups
        .iter()
        .map(|g| g.paths.iter().collect::<Vec<&Path>>())
        .filter(|p| p.len() != 0) // filter out all groups with zero paths
        .collect();
    // println!("groups_paths={:?}", groups_paths);


    // from each parallel state save the resulting local maximum as (cost, state)
    let mut local_maxima: Vec<(u64, SelectionState)> = Vec::with_capacity(n_runs as usize);


    for run in 0..n_runs {

        println!("[run={}/{}]", run+1, n_runs);

        // choose random configuration as initial state
        let initial_state = SelectionState::generate_random_state(&groups_paths);

        println!(
            "\tinitial cost={}",
            initial_state.get_cost()
        );

        let mut local_maximum = (initial_state.get_cost(), initial_state);


        for j in 0..n_iterations {
            // search local maximum from this initial configuration

            print!("\t[iteration={}/{}]: ", j+1, n_iterations);
            
            let mut neighbors = local_maximum.1.generate_neighbors();

            // for (neighbor_index, neighbor) in neighbors.iter().enumerate() {
            //     println!("[{}]: neighbor={:?}, cost={}", neighbor_index, neighbor.1.groups_paths_selection, neighbor.0);
            // }

            if neighbors.len() == 0 || neighbors[0].0 >= local_maximum.0 {
                println!("reached local maximum -> stopping search");

                // as we won't find any better solution -> early exit loop
                break;
            }

            println!("found new local maximum neighbor cost={}", neighbors[0].0);

            // set as new local maximum
            neighbors.reverse();
            local_maximum = neighbors.pop().unwrap();
        }

        local_maxima.push(local_maximum);


    }

    local_maxima.sort_unstable_by_key(|(cost, _)| *cost);
    println!("local maxima: {:?}", local_maxima.iter().map(|(cost, state)| cost).collect::<Vec<_>>());

    // // stores the index of the currently selected path in each group
    // let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());
}
