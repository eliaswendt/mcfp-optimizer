use petgraph::graph::DiGraph;
use rand::Rng;

use crate::model::{graph_weigth::{TimetableEdge, TimetableNode}, group::Group, path::Path};

pub mod simulated_annealing;
pub mod randomized_hillclimb;
pub mod simulated_annealing_elias;


/// formalizing a system state
/// by storing the indices of the currently selected path for each group
#[derive(Debug, Clone)]
pub struct SelectionState<'a> {
    groups: &'a Vec<Group>,
    
    // groups_paths: &'a Vec<Vec<&'a Path>>,
    pub groups_paths_selection: Vec<usize>, // array of indices (specifies selected path for each group)
}

impl<'a> SelectionState<'a> {

    pub fn generate_random_state(groups: &'a Vec<Group>) -> Self {
        let mut rng = rand::thread_rng();

        let mut groups_paths_selection = Vec::with_capacity(groups.len());

        for group in groups.iter() {
            groups_paths_selection.push(rng.gen::<usize>() % group.paths.len());
        }

        Self {
            groups,
            groups_paths_selection,
        }
    }

    /// generates a vec of neighbor states
    /// generate new states, so that each neighbor only differs in selected path of one group
    pub fn generate_group_neighbors(&self) -> Vec<Self> {

        let mut neighbors = Vec::with_capacity(self.groups.len() * 10);

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {
            // for each group add state with all possible paths for current group
            let n_paths_of_group = self.groups[group_index].paths.len();
            for path_index in 0..n_paths_of_group {
                if path_index == self.groups_paths_selection[group_index] {
                    // skip initial path_index
                    continue;
                }

                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] = path_index; // set current path_index as selected

                let selection_state = Self {
                    groups: self.groups,
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
        let mut neighbors = Vec::with_capacity(self.groups.len() * 2);

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {
            // create state with index decremented by one
            if self.groups_paths_selection[group_index] != 0 {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] -= 1;

                let selection_state = Self {
                    groups: self.groups,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            if self.groups_paths_selection[group_index] != self.groups[group_index].paths.len() - 1
            {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] += 1;

                let selection_state = Self {
                    groups: self.groups,
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
            self.groups[group_index].paths[*path_index].strain(graph);
        }
        // println!("first took {}ms", start.elapsed().as_millis());



        // second: calculate sum of all path's utilization costs
        // let start = Instant::now();
        let mut cost_sum = 0;
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            cost_sum += self.groups[group_index].paths[*path_index].utilization_cost(graph);
        }
        // println!("second took {}ms", start.elapsed().as_millis());



        // third: relieve all selected paths to TimetableGraph
        // let start = Instant::now();
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve(graph);
        }
        // println!("thrid took {}ms", start.elapsed().as_millis());


        cost_sum
    }
}
