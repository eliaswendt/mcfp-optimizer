use petgraph::graph::DiGraph;
use rand::Rng;

use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::Path};

pub mod simulated_annealing;
pub mod randomized_hillclimb;
pub mod simulated_annealing_elias;


/// formalizing a system state
/// by storing the indices of the currently selected path for each group
#[derive(Debug, Clone)]
pub struct SelectionState<'a> {
    groups: &'a Vec<Group>,
    
    cost: u64,
    // groups_paths: &'a Vec<Vec<&'a Path>>,
    pub groups_paths_selection: Vec<usize>, // array of indices (specifies selected path for each group)
}

impl<'a> SelectionState<'a> {

    pub fn generate_random_state(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>) -> Self {
        let mut rng = rand::thread_rng();

        let mut groups_paths_selection = Vec::with_capacity(groups.len());

        for group in groups.iter() {
            groups_paths_selection.push(rng.gen::<usize>() % group.paths.len());
        }


        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            groups[group_index].paths[*path_index].strain_to_graph(graph);
        }

        // second: calculate sum of all path's utilization costs
        let mut cost_sum = 0;
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            cost_sum += groups[group_index].paths[*path_index].utilization_cost(graph);
        }

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph);
        }


        Self {
            groups,
            cost: cost_sum,
            groups_paths_selection,
        }
    }

    /// generates a vec of neighbor states
    /// generate new states, so that each neighbor only differs in selected path of one group
    pub fn generate_group_neighbors(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) -> Vec<Self> {
        
        let mut neighbors = Vec::with_capacity(self.groups.len() * 10);

        // for faster cost calculation now strain all actual selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].strain_to_graph(graph);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {

            let actual_selected_path_index = self.groups_paths_selection[group_index];

            // relieve the actual selected path of current group
            self.groups[group_index].paths[actual_selected_path_index].relieve_from_graph(graph);

            // for each group add state with all possible paths for current group
            let n_paths_of_group = self.groups[group_index].paths.len();
            for path_index in 0..n_paths_of_group {

                if path_index == self.groups_paths_selection[group_index] {
                    // skip initial path_index
                    continue;
                }


                // calculate cost for when choosing path_index instead of actual_path_index
                let mut cost_sum = 0;
                self.groups[group_index].paths[path_index].strain_to_graph(graph);
                for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
                    cost_sum += self.groups[group_index].paths[*path_index].utilization_cost(graph);
                }
                self.groups[group_index].paths[path_index].relieve_from_graph(graph);
                

                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] = path_index; // set current path_index as selected

                let selection_state = Self {
                    groups: self.groups,

                    cost: cost_sum,
                    groups_paths_selection: groups_paths_selection_clone,
                };


                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[actual_selected_path_index].strain_to_graph(graph);
        }      
        
        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve_from_graph(graph);
        }

        neighbors
    }

    /// generates a vec of neighbor states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    /// this function also efficiently calculates the cost during creation of path configurations
    pub fn generate_direct_neighbors(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) -> Vec<Self> {

        let mut neighbors = Vec::with_capacity(self.groups.len() * 2);

        // for faster cost calculation now strain all actual selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].strain_to_graph(graph);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {

            // fetch selected path index of current group
            let actual_selected_path_index = self.groups_paths_selection[group_index];

            // relieve the actual selected path of current group
            self.groups[group_index].paths[actual_selected_path_index].relieve_from_graph(graph);


            // create state with index decremented by one
            if actual_selected_path_index != 0 {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] -= 1;

                let mut cost_sum = 0;
                self.groups[group_index].paths[actual_selected_path_index - 1].strain_to_graph(graph);
                for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
                    cost_sum += self.groups[group_index].paths[*path_index].utilization_cost(graph);
                }
                self.groups[group_index].paths[actual_selected_path_index - 1].relieve_from_graph(graph);


                let selection_state = Self {
                    groups: self.groups,

                    cost: cost_sum,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            if actual_selected_path_index != self.groups[group_index].paths.len() - 1
            {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] += 1;

                let mut cost_sum = 0;
                self.groups[group_index].paths[actual_selected_path_index + 1].strain_to_graph(graph);
                for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
                    cost_sum += self.groups[group_index].paths[*path_index].utilization_cost(graph);
                }
                self.groups[group_index].paths[actual_selected_path_index + 1].relieve_from_graph(graph);


                let selection_state = Self {
                    groups: self.groups,

                    cost: cost_sum,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[actual_selected_path_index].strain_to_graph(graph);
        }

        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve_from_graph(graph);
        }

        neighbors
    }
}
