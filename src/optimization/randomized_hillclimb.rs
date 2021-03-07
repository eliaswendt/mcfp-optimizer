use std::{iter::Map, time::Instant};

use petgraph::graph::DiGraph;
use rand::Rng;

use crate::model::{
    group::Group,
    path::Path,
    timetable_graph::{TimetableEdge, TimetableNode},
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
    /// generate new states, so that each neighbor only differs in selected path of one group
    pub fn generate_neighbors(&self) -> Vec<Self> {

        let mut neighbors = Vec::with_capacity(self.groups_paths.len() * 10);

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {
            // for each group add state with all possible paths for current group
            let n_paths_of_group = self.groups_paths[group_index].len();
            for path_index in 0..n_paths_of_group {
                if path_index == self.groups_paths_selection[group_index] {
                    // skip initial path_index
                    continue;
                }

                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] = path_index; // set current path_index as selected

                let selection_state = Self {
                    groups_paths: self.groups_paths,
                    groups_paths_selection: groups_paths_selection_clone,
                };


                neighbors.push(selection_state);
            }
        }        

        neighbors
    }

    /// generates a vec of neighbor states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    pub fn generate_direct_neighbors(&self) -> Vec<Self> {
        let mut neighbors = Vec::with_capacity(self.groups_paths.len() * 2);

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

                neighbors.push(selection_state);
            }

            if self.groups_paths_selection[group_index] != self.groups_paths[group_index].len() - 1
            {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] += 1;

                let selection_state = Self {
                    groups_paths: self.groups_paths,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }
        }

        neighbors
    }

    #[inline]
    pub fn get_cost(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) -> u64 {

        // first: strain all selected paths to TimetableGraph
        // let start = Instant::now();
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups_paths[group_index][*path_index].strain(graph);
        }
        // println!("first took {}ms", start.elapsed().as_millis());



        // second: calculate sum of all path's utilization costs
        // let start = Instant::now();
        let mut cost_sum = 0;
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            cost_sum += self.groups_paths[group_index][*path_index].utilization_cost(graph);
        }
        // println!("second took {}ms", start.elapsed().as_millis());



        // third: relieve all selected paths to TimetableGraph
        // let start = Instant::now();
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups_paths[group_index][*path_index].relieve(graph);
        }
        // println!("thrid took {}ms", start.elapsed().as_millis());


        cost_sum
    }
}

/// perform a single Hill Climbing Step
pub fn randomized_hillclimb(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
    n_restarts: u64,       // number of "parallel" hill-climb searches
    max_n_iterations: u64, // number of iterations to improve result
) {
    println!(
        "randomized_hillclimb(n_runs={}, n_iterations={})",
        n_restarts, max_n_iterations
    );

    let groups_paths: Vec<Vec<&Path>> = groups
        .iter()
        .map(|g| g.paths.iter().collect::<Vec<&Path>>())
        .filter(|p| p.len() != 0) // filter out all groups with zero paths
        .collect();
    // println!("groups_paths={:?}", groups_paths);

    // from each parallel state save the resulting local maximum as (cost, state)
    let mut local_minima: Vec<(u64, SelectionState)> = Vec::with_capacity(n_restarts as usize);

    for run in 0..n_restarts {
        // choose random configuration as initial state
        let initial_state = SelectionState::generate_random_state(&groups_paths);
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

    // // stores the index of the currently selected path in each group
    // let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());
}
