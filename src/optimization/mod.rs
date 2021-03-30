use std::{
    collections::HashSet,
    fmt,
    fs::File,
    io::{BufWriter, Write},
};

use indexmap::IndexSet;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use rand::{prelude::ThreadRng, Rng};

use crate::model::{
    graph_weight::{TimetableEdge, TimetableNode},
    group::Group,
    path::{self, Path},
};

pub mod randomized_best;
pub mod randomized_hillclimb;
pub mod simulated_annealing;
pub(crate) mod simulated_annealing_on_path;

/// formalizing a system state
/// by storing the indices of the currently selected path for each group
#[derive(Debug, Clone)]
pub struct SelectionState<'a> {
    pub groups: &'a Vec<Group>,
    pub cost: i64, // total cost of this path selection
    pub strained_edges_cost: i64,
    pub travel_cost: i64,
    pub travel_delay_cost: i64,
    pub groups_path_index: Vec<usize>, // array of indices (specifies selected path for each group)
}

impl fmt::Display for SelectionState<'_> {
    // This trait  `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.

        for (group, group_path_index) in self.groups.iter().zip(self.groups_path_index.iter()) {
            write!(
                f,
                "\n[group_id={}]: {} ({}) -> {} ({}), travel_cost={}, travel_delay={}",
                group.id,
                group.start_station_id,
                group.departure_time,
                group.destination_station_id,
                group.arrival_time,
                group.paths[*group_path_index].travel_cost(),
                group.paths[*group_path_index].travel_delay()
            )?
        }

        Ok(())
    }
}

impl<'a> SelectionState<'a> {
    pub fn calculate_cost_of_strained_edges(
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        strained_edges: &HashSet<EdgeIndex>,
    ) -> u64 {
        strained_edges
            .iter()
            .map(|e| graph[*e].utilization_cost())
            .sum()
    }

    pub fn calculate_total_travel_cost_paths(
        groups: &Vec<Group>,
        groups_path_index: &Vec<usize>,
    ) -> i64 {
        groups
            .iter()
            .zip(groups_path_index.iter())
            .map(|(group, group_path_index)| group.paths[*group_path_index].travel_cost())
            .sum::<u64>() as i64
    }

    pub fn calculate_total_travel_delay_cost_paths(
        groups: &Vec<Group>,
        groups_path_index: &Vec<usize>,
    ) -> i64 {
        groups
            .iter()
            .zip(groups_path_index.iter())
            .map(|(group, group_path_index)| group.paths[*group_path_index].travel_delay())
            .sum()
    }

    pub fn save_groups_to_csv(
        &self,
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        filepath: &str,
    ) {
        let mut writer = BufWriter::new(
            File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath)),
        );

        writer
            .write("group_id|planned_time|real_time|travel_cost|delay|delay_in_%|waiting_time|in_trip_time|walks|path\n".as_bytes())
            .unwrap();
        for (group_index, group) in self.groups.iter().enumerate() {
            let path_index = self.groups_path_index[group_index];
            let path = &group.paths[path_index];
            let last_node_index = graph.edge_endpoints(*path.edges.last().unwrap()).unwrap().1;

            let planned = group.arrival_time - group.departure_time;
            let real = graph.node_weight(last_node_index).unwrap().time() - group.departure_time;
            let travel_cost = path.travel_cost();
            let delay = real as i64 - planned as i64;
            let delay_p = 100 * delay / planned as i64;
            let waiting_time = path.get_waiting_time(graph);
            let in_trip_time = path.get_in_trip_time(graph);
            let walks = path.get_walks(graph);
            let path_string = path.to_string(graph);

            writer
                .write(
                    format!(
                        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                        group.id,
                        planned,
                        real,
                        travel_cost,
                        delay,
                        delay_p,
                        waiting_time,
                        in_trip_time,
                        walks,
                        path_string
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
    }

    pub fn save_strained_trip_edges_to_csv(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        filepath: &str,
    ) {
        let mut writer = BufWriter::new(
            File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath)),
        );

        writer
            .write("edge_index|duration|capacity|utilization\n".as_bytes())
            .unwrap();

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            let path = &self.groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        // find a random overcrowded edge
        for edge_index in &strained_edges {
            let edge = &graph[*edge_index];
            if edge.is_trip() {
                writer
                    .write(
                        format!(
                            "{:?}|{}|{}|{}\n",
                            edge_index,
                            edge.duration(),
                            edge.capacity(),
                            edge.utilization()
                        )
                        .as_bytes(),
                    )
                    .unwrap();
            }
        }

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            self.groups[group_index].paths[*path_index]
                .relieve_from_graph(graph, &mut strained_edges);
        }
    }

    pub fn generate_random_state(
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        groups: &'a Vec<Group>,
    ) -> Self {
        let mut rng = rand::thread_rng();

        let mut groups_path_index = Vec::with_capacity(groups.len());

        for group in groups.iter() {
            // iterate over all groups and generate a random index (in range of #paths of current group)
            groups_path_index.push(rng.gen::<usize>() % group.paths.len());
        }

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_path_index.iter().enumerate() {
            let path = &groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let strained_edges_cost =
            Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
        let travel_cost = Self::calculate_total_travel_cost_paths(groups, &groups_path_index);
        let travel_delay_cost =
            Self::calculate_total_travel_delay_cost_paths(groups, &groups_path_index);
        let cost = strained_edges_cost + travel_cost + travel_delay_cost;

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_path_index.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups,
            cost,
            strained_edges_cost,
            travel_cost,
            travel_delay_cost,
            groups_path_index,
        }
    }

    pub fn generate_state_with_best_path_per_group(
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        groups: &'a Vec<Group>,
    ) -> Self {
        // each group's path vec is sorted (lowest cost first) -> set all selected indices to zero
        let groups_path_index = vec![0; groups.len()];

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, selected_path_index) in groups_path_index.iter().enumerate() {
            let path = &groups[group_index].paths[*selected_path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let strained_edges_cost =
            Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;

        let travel_cost = Self::calculate_total_travel_cost_paths(groups, &groups_path_index);
        let travel_delay_cost =
            Self::calculate_total_travel_delay_cost_paths(groups, &groups_path_index);
        let cost = strained_edges_cost + travel_cost + travel_delay_cost;

        // third: relieve all selected paths from TimetableGraph
        for (group_index, path_index) in groups_path_index.iter().enumerate() {
            groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups,
            cost,
            strained_edges_cost,
            travel_cost,
            travel_delay_cost,
            groups_path_index,
        }
    }

    /// generates a vec of neighbor states
    /// generate new states, so that each neighbor only differs in selected path of one group
    pub fn all_group_neighbors(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    ) -> Vec<Self> {
        let mut neighbors = Vec::with_capacity(self.groups.len() * 10);

        // stores all edges currently strained to the graph
        let mut strained_edges = HashSet::new();

        // for faster cost calculation now strain all actual selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            let path = &self.groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_path_index.len() {
            let self_selected_path_index = self.groups_path_index[group_index];

            // relieve the self selected path of current group
            self.groups[group_index].paths[self_selected_path_index]
                .relieve_from_graph(graph, &mut strained_edges);

            // for each group add state with all possible paths for current group
            let n_paths_of_group = self.groups[group_index].paths.len();
            for path_index in 0..n_paths_of_group {
                if path_index == self.groups_path_index[group_index] {
                    // skip self selected path_index
                    continue;
                }

                // strain new path (for current group) to graph
                self.groups[group_index].paths[path_index]
                    .strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let strained_edges_cost =
                    Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
                // relieve new path from graph
                self.groups[group_index].paths[path_index]
                    .relieve_from_graph(graph, &mut strained_edges);

                let mut groups_paths_selection_clone = self.groups_path_index.clone();
                groups_paths_selection_clone[group_index] = path_index; // set current path_index as selected

                let travel_cost = Self::calculate_total_travel_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let travel_delay_cost = Self::calculate_total_travel_delay_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let cost = strained_edges_cost + travel_cost + travel_delay_cost;

                let selection_state = Self {
                    groups: self.groups,
                    cost,
                    strained_edges_cost,
                    travel_cost,
                    travel_delay_cost,
                    groups_path_index: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[self_selected_path_index]
                .strain_to_graph(graph, &mut strained_edges);
        }

        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            self.groups[group_index].paths[*path_index]
                .relieve_from_graph(graph, &mut strained_edges);
        }

        neighbors
    }

    /// generates a vec of neighbor states
    /// create two new states per selected_path_index -> one with the one-lower index (if > 0) + one with the one-higher index (if in bounds)
    /// this function also efficiently calculates the cost during creation of path configurations
    pub fn all_direct_neighbors(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    ) -> Vec<Self> {
        let mut neighbors = Vec::with_capacity(self.groups.len() * 2);

        // stores all edges currently strained to the graph
        let mut strained_edges = HashSet::new();

        // for faster cost calculation now strain all self selected paths to the graph and only switch paths for the groups we are currently working on
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            self.groups[group_index].paths[*path_index].strain_to_graph(graph, &mut strained_edges);
        }

        // iterate over all groups_paths_selection
        for group_index in 0..self.groups_path_index.len() {
            // fetch selected path index of current group
            let actual_selected_path_index = self.groups_path_index[group_index];

            // relieve the actual selected path of current group
            self.groups[group_index].paths[actual_selected_path_index]
                .relieve_from_graph(graph, &mut strained_edges);

            // create state with index decremented by one
            if actual_selected_path_index != 0 {
                let mut groups_paths_selection_clone = self.groups_path_index.clone();
                groups_paths_selection_clone[group_index] -= 1;

                // strain new path (for current group) to graph
                self.groups[group_index].paths[actual_selected_path_index - 1]
                    .strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let strained_edges_cost =
                    Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
                // relieve new path from graph
                self.groups[group_index].paths[actual_selected_path_index - 1]
                    .relieve_from_graph(graph, &mut strained_edges);

                let travel_cost = Self::calculate_total_travel_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let travel_delay_cost = Self::calculate_total_travel_delay_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let cost = strained_edges_cost + travel_cost + travel_delay_cost;

                let selection_state = Self {
                    groups: self.groups,
                    cost,
                    strained_edges_cost,
                    travel_cost,
                    travel_delay_cost,
                    groups_path_index: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            if actual_selected_path_index != self.groups[group_index].paths.len() - 1 {
                let mut groups_paths_selection_clone = self.groups_path_index.clone();
                groups_paths_selection_clone[group_index] += 1;

                // strain new path (for current group) to graph
                self.groups[group_index].paths[actual_selected_path_index + 1]
                    .strain_to_graph(graph, &mut strained_edges);
                // calculate cost of all strained edges
                let strained_edges_cost =
                    Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
                // relieve new path from graph
                self.groups[group_index].paths[actual_selected_path_index + 1]
                    .relieve_from_graph(graph, &mut strained_edges);

                let travel_cost = Self::calculate_total_travel_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let travel_delay_cost = Self::calculate_total_travel_delay_cost_paths(
                    &self.groups,
                    &groups_paths_selection_clone,
                );
                let cost = strained_edges_cost + travel_cost + travel_delay_cost;

                let selection_state = Self {
                    groups: self.groups,
                    cost,
                    strained_edges_cost,
                    travel_cost,
                    travel_delay_cost,
                    groups_path_index: groups_paths_selection_clone,
                };

                neighbors.push(selection_state);
            }

            // re-add the actually selected path for current group to graph
            self.groups[group_index].paths[actual_selected_path_index]
                .strain_to_graph(graph, &mut strained_edges);
        }

        // at the beginning of the function we strained all actual selected paths to the graph
        // before returning relieve them
        for (group_index, path_index) in self.groups_path_index.iter().enumerate() {
            self.groups[group_index].paths[*path_index]
                .relieve_from_graph(graph, &mut strained_edges);
        }

        neighbors
    }

    /// generate a single group neighbor
    /// if not specified, select a random path for a random group
    pub fn random_group_neighbor(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        rng: &mut ThreadRng,

        group_index_option: Option<usize>,
        path_index_option: Option<usize>,
    ) -> Self {
        let group_index = match group_index_option {
            Some(group_index) => group_index,
            None => rng.gen::<usize>() % self.groups.len(),
        };

        let path_index = match path_index_option {
            Some(path_index) => path_index,
            None => rng.gen::<usize>() % self.groups[group_index].paths.len(),
        };

        let mut groups_paths_selection = self.groups_path_index.clone();
        groups_paths_selection[group_index] = path_index;

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            let path = &self.groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let strained_edges_cost =
            Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
        let travel_cost =
            Self::calculate_total_travel_cost_paths(self.groups, &groups_paths_selection);
        let travel_delay_cost =
            Self::calculate_total_travel_delay_cost_paths(self.groups, &groups_paths_selection);
        let cost = strained_edges_cost + travel_cost + travel_delay_cost;

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            self.groups[group_index].paths[*path_index]
                .relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups: self.groups,
            cost,
            strained_edges_cost,
            travel_cost,
            travel_delay_cost,
            groups_path_index: groups_paths_selection,
        }
    }

    pub fn group_neighbor_from_group_and_path(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        groups: &mut Vec<Group>,
        group_index: usize,
        path_index: usize,
    ) -> Self {
        let mut groups_paths_selection = self.groups_path_index.clone();
        groups_paths_selection[group_index] = path_index;

        let mut strained_edges: HashSet<EdgeIndex> = HashSet::new();

        // first: strain all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            let path = &groups[group_index].paths[*path_index];
            path.strain_to_graph(graph, &mut strained_edges);
        }

        let strained_edges_cost =
            Self::calculate_cost_of_strained_edges(graph, &strained_edges) as i64;
        let travel_cost = Self::calculate_total_travel_cost_paths(groups, &groups_paths_selection);
        let travel_delay_cost =
            Self::calculate_total_travel_delay_cost_paths(groups, &groups_paths_selection);
        let cost = strained_edges_cost + travel_cost + travel_delay_cost;

        // third: relieve all selected paths to TimetableGraph
        for (group_index, path_index) in groups_paths_selection.iter().enumerate() {
            &groups[group_index].paths[*path_index].relieve_from_graph(graph, &mut strained_edges);
        }

        Self {
            groups: self.groups,
            cost,
            strained_edges_cost,
            travel_cost,
            travel_delay_cost,
            groups_path_index: groups_paths_selection,
        }
    }

    pub fn get_random_overcrowded_edge_with_groups(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        groups: &mut Vec<Group>,
        rng: &mut ThreadRng,
    ) -> (EdgeIndex, Vec<usize>) {
        let groups_paths_selection = self.groups_path_index.clone();

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

        print!("num_edges={}, ", edges.len());

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
        let path_index = self.groups_path_index[random_group];
        let path = &groups[random_group].paths[path_index].clone();

        // find all edges before chosen overcrowded edge in path
        let mut edges_before_edge = IndexSet::new();

        for edge_index in &path.edges {
            if *edge_index == edge {
                break;
            }
            edges_before_edge.insert(*edge_index);
        }

        // start with edge before selected overcrowded edge
        edges_before_edge.reverse();

        let end = graph.edge_endpoints(*path.edges.last().unwrap()).unwrap().1;

        for edge_index in edges_before_edge.clone() {

            // get start node
            let (start, _) = graph.edge_endpoints(edge_index).unwrap();

            // let edge_sets = path::Path::all_paths_iddfs(
            //     graph,
            //     start,
            //     groups[random_group].destination_station_id,
            //     100,
    
            //     2 * groups[random_group].arrival_time - graph[start].time() + 60,
            //     &vec![
            //         50
            //     ],
            // );

            // transform each edge_set into a full Path object
            let mut possible_paths: Vec<Path>;
            // possible_paths = edge_sets
            // .into_iter()
            // .filter(|edge_set| edge_set.len() != 0) // filter out empty edge_sets (paths that don't have a single edge)
            // .map(|edge_set| Path::new(graph, edge_set, groups[random_group].passengers, groups[random_group].arrival_time))
            // .collect();

            // if possible_paths.len() == 0 {
            // get possible paths from current start node to end node
            possible_paths = path::Path::dfs_visitor_search(
                graph,
                start,
                graph[end].station_id(),
                groups[random_group].passengers as u64,
                groups[random_group].arrival_time,
                0,
            );
            // }
            //println!("{}", possible_paths.len());

            // if we have more paths as before
            if possible_paths.len() > 3 {
                let path_index;
                // match rng {
                //     Some(rng) => path_index = rng.gen::<usize>() % possible_paths.len(),
                //     None => {
                //         possible_paths.sort_unstable_by_key(|p| p.cost());
                //         path_index = 0;
                //     }
                // }
                possible_paths.sort_unstable_by_key(|p| p.cost());
                path_index = 0;

                let mut new_path = Vec::new();

                // build new path completely
                edges_before_edge.reverse();
                for next_edge_index in edges_before_edge.iter() {
                    if *next_edge_index == edge_index {
                        break;
                    }
                    new_path.push(*next_edge_index);
                }
                for next_edge_index in possible_paths[path_index].edges.iter() {
                    new_path.push(*next_edge_index);
                }

                return (
                    random_group,
                    Some(Path::new(
                        graph,
                        new_path,
                        groups[random_group].passengers as u64,
                        groups[random_group].arrival_time,
                    )),
                );
            }
        }

        (random_group, None)
    }
}
