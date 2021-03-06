use petgraph::graph::DiGraph;

use crate::model::{group::Group, TimetableEdge, TimetableNode};

/// formalizing a system state
/// by storing indices of currently selected path for each group
struct State {
    selected_path_indices: Vec<usize>,
    upper_index_bounds: Vec<usize> // highest index per group
}

impl State {

    /// generates a vec of child states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    pub fn generate_child_states(&self) -> Vec<State> {
        let children = Vec::with_capacity(self.selected_path_indices.len());

        // iterate over all selected_path_indices
        for i in 0..self.selected_path_indices.len() {

            // create state with index decremented by one
            if self.selected_path_indices[i]

            let selected_path_indices = self.selected_path_indices.clone();
            selected_path_indices[i] = 

            children.push(State {
                selected_path_indices: self.selected_path_indices.clone()
            });
        }

        children
    }
}

/// perform a single Hill Climbing Step
pub fn hill_climb_step(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
    beam_size: u64, // number of "parallel" hill-climb searches
) {

    for group in groups {
        println!("[group={}]", group.id);
        for path in group.paths.iter() {
            println!("\t{:?}", path);
        }
    }

    // // stores the index of the currently selected path in each group
    // let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());
}




