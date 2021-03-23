use std::collections::HashSet;

use indexmap::IndexSet;
use petgraph::graph::{DiGraph, EdgeIndex};
use rand::{Rng, prelude::ThreadRng};

use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::{self, Path}};

pub mod simulated_annealing;
pub mod randomized_hillclimb;
pub mod simulated_annealing_elias;
pub mod randomized_best;
pub(crate) mod simulated_annealing_on_path;


/// formalizing a system state
/// by storing the indices of the currently selected path for each group
#[derive(Debug, Clone)]
pub struct SelectionState<'a> {
    pub groups: &'a Vec<Group>,    
    pub cost: u64, // cost of this path selection
    pub groups_paths_selection: Vec<usize>, // array of indices (specifies selected path for each group)
}

impl<'a> SelectionState<'a> {

    pub fn calculate_cost_of_strained_edges(graph: &mut DiGraph<TimetableNode, TimetableEdge>, strained_edges: &HashSet<EdgeIndex>) -> u64 {
        strained_edges.iter().map(|e| graph[*e].utilization_cost()).sum()
    }

    pub fn generate_random_state(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>) -> Self {
        let mut rng = rand::thread_rng();

        let mut groups_paths_selection = Vec::with_capacity(groups.len());

        for group in groups.iter() {
            // iterate over all groups and generate a random index (in range of #paths of current group)
            groups_paths_selection.push(
                rng.gen::<usize>() % group.paths.len()
            );
        }

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {

            let path = &groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups,
            cost,
            groups_paths_selection,
        }
    }

    /// generates a vec of neighbor states
    /// generate new states, so that each neighbor only differs in selected path of one group
    pub fn all_group_neighbors(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) -> Vec<Self> {
        
        let mut neighbors = Vec::with_capacity(self.groups.len() * 10);

        // stores all edges currently strained to the graph
        let mut strained_edges = HashSet::new();

        // for faster cost calculation now strain all actual selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].strain_to_graph(graph, &mut strained_edges);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {

            let actual_selected_path_index = self.groups_paths_selection[group_index];

            // relieve the actual selected path of current group
            self.groups[group_index].paths[actual_selected_path_index].relieve_from_graph(graph, &mut strained_edges);

            // for each group add state with all possible paths for current group
            let n_paths_of_group = self.groups[group_index].paths.len();
            for path_index in 0..n_paths_of_group {

                if path_index == self.groups_paths_selection[group_index] {
                    // skip initial path_index
                    continue;
                }

                // strain new path (for current group) to graph
                self.groups[group_index].paths[path_index].strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);
                // relieve new path from graph
                self.groups[group_index].paths[path_index].relieve_from_graph(graph, &mut strained_edges);
                

                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] = path_index; // set current path_index as selected

                let selection_state = Self {
                    groups: self.groups,

                    cost,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[actual_selected_path_index].strain_to_graph(graph, &mut strained_edges);
        }      
        
        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        neighbors
    }

    /// generates a vec of neighbor states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    /// this function also efficiently calculates the cost during creation of path configurations
    pub fn all_direct_neighbors(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) -> Vec<Self> {

        let mut neighbors = Vec::with_capacity(self.groups.len() * 2);

        // stores all edges currently strained to the graph
        let mut strained_edges = HashSet::new();

        // for faster cost calculation now strain all actual selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].strain_to_graph(graph, &mut strained_edges);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_paths_selection.len() {

            // fetch selected path index of current group
            let actual_selected_path_index = self.groups_paths_selection[group_index];

            // relieve the actual selected path of current group
            self.groups[group_index].paths[actual_selected_path_index].relieve_from_graph(graph, &mut strained_edges);


            // create state with index decremented by one
            if actual_selected_path_index != 0 {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] -= 1;


                // strain new path (for current group) to graph
                self.groups[group_index].paths[actual_selected_path_index - 1].strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);
                // relieve new path from graph
                self.groups[group_index].paths[actual_selected_path_index - 1].relieve_from_graph(graph, &mut strained_edges);


                let selection_state = Self {
                    groups: self.groups,

                    cost,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            if actual_selected_path_index != self.groups[group_index].paths.len() - 1
            {
                let mut groups_paths_selection_clone = self.groups_paths_selection.clone();
                groups_paths_selection_clone[group_index] += 1;

                // strain new path (for current group) to graph
                self.groups[group_index].paths[actual_selected_path_index + 1].strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);
                // relieve new path from graph
                self.groups[group_index].paths[actual_selected_path_index + 1].relieve_from_graph(graph, &mut strained_edges);

                let selection_state = Self {
                    groups: self.groups,

                    cost,
                    groups_paths_selection: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[actual_selected_path_index].strain_to_graph(graph, &mut strained_edges);
        }

        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        neighbors
    }

    pub fn random_group_neighbor(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>, rng: &mut ThreadRng) -> Self {

        let random_group_index = rng.gen::<usize>() % self.groups.len();
        let random_path_index = rng.gen::<usize>() % self.groups[random_group_index].paths.len();

        let mut groups_paths_selection = self.groups_paths_selection.clone();
        groups_paths_selection[random_group_index] = random_path_index;

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {

            let path = &self.groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups: self.groups,
            cost,
            groups_paths_selection,
        }
    }

    pub fn group_neighbor_from_group_and_path(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &mut Vec<Group>, group_index: usize, path_index: usize) -> Self {

        let mut groups_paths_selection = self.groups_paths_selection.clone();
        groups_paths_selection[group_index] = path_index;

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {

            let path = &groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let cost = Self::calculate_cost_of_strained_edges(graph, &strained_edges);

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups: self.groups,
            cost,
            groups_paths_selection,
        }
    }

    pub fn get_random_overcrowded_edge_with_groups(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &mut Vec<Group>, rng: &mut ThreadRng) -> (EdgeIndex, Vec<usize>) {

        let groups_paths_selection = self.groups_paths_selection.clone();

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {

            let path = &groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let mut edges = Vec::new(); 

        // find a random overcrowded edge
        for edge_index in strained_edges.clone() {
            if graph[edge_index].utilization_cost() > 0 {
                edges.push(edge_index);
            }
        }

        let random_edge_index = rng.gen::<usize>() % edges.len();
        let random_edge = edges[random_edge_index];

        let mut group_indices = Vec::new();

        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {

            let path = &groups[group_index].paths[*path_index];
            if path.edges.contains(&random_edge) {
                group_indices.push(group_index);
            }
        }

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        (random_edge, group_indices)
    }

    pub fn find_detour_for_random_group(
        &self, 
        graph: &mut DiGraph<TimetableNode, TimetableEdge>, 
        groups: &mut Vec<Group>,
        group_indices: Vec<usize>,
        edge: EdgeIndex,
        rng: &mut ThreadRng, 
    ) -> (usize, Option<Path>) {

        // select random group for detour
        let random_group_index = rng.gen::<usize>() % group_indices.len();
        let random_group = group_indices[random_group_index];

        // get path of the selected random group
        let path_index = self.groups_paths_selection[random_group];
        let path = groups[random_group].paths[path_index].clone();

        // find all edges before chosen overcrowded edge in path
        let mut indices_before_edge = Vec::new();

        for edge_index in &path.edges {
            if *edge_index == edge {
                break;
            }
            indices_before_edge.push(edge_index)
        }
        
        // start with edge before selected overcrowded edge
        indices_before_edge.reverse();

        // find end node of path: todo
        //let (_, end) = graph.edge_endpoints(*edge).unwrap();
        let (_, end) = graph.edge_endpoints(*path.edges.last().unwrap()).unwrap();

        for edge_index in indices_before_edge.clone() {

            // get start node 
            let (start, _) = graph.edge_endpoints(*edge_index).unwrap();

            // get possible paths from current start node to end node
            let possible_paths = path::Path::dfs_visitor_search(
                graph,
                start,
                end,
                groups[random_group].passengers as u64,
                groups[random_group].arrival,
                0
            );

            // if we have more paths as before 
            if possible_paths.len() > 2 {              
                let random_path_index = rng.gen::<usize>() % possible_paths.len();

                let mut new_path = IndexSet::new();

                // build new path completely
                indices_before_edge.reverse();
                for next_edge_index in indices_before_edge {
                    if *next_edge_index == *edge_index {
                        break;
                    }
                    new_path.insert(*next_edge_index);
                }
                for next_edge_index in possible_paths[random_path_index].edges.iter() {
                    new_path.insert(*next_edge_index);
                }

                return (random_group, Some(Path::new(
                    graph,
                    new_path,
                    groups[random_group].passengers as u64,
                    groups[random_group].arrival
                )))
            }
        }
 
        (random_group, None)
    }
}
