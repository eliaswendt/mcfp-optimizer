use indexmap::IndexSet;
use petgraph::{
    dot::Dot,
    graph::{DiGraph, EdgeIndex, NodeIndex},
    visit::{depth_first_search, Control, DfsEvent, EdgeRef},
    EdgeDirection::Outgoing,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt,
    fs::File,
    io::{self, BufWriter, Write},
    iter::from_fn,
    ops::Add,
};

use super::{TimetableEdge, TimetableNode};

#[derive(Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Path {
    travel_cost: u64,     // cost for this path
    travel_duration: u64, // duration of this path
    travel_delay: i64,    // time between planned and real arrival
    utilization: u64,     // number of passengers

    pub edges: IndexSet<EdgeIndex>,
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost().cmp(&other.cost())
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.cost() == other.cost()
    }
}

impl Path {
    /// edges must not be empty
    pub fn new(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        edges: Vec<EdgeIndex>,
        utilization: u64,
        planned_arrival_time: u64,
    ) -> Self {
        let mut travel_cost: u64 = 0;
        let mut duration: u64 = 0;

        for edge in edges.iter() {
            let edge_weight = &graph[*edge];
            travel_cost += edge_weight.travel_cost();
            duration += edge_weight.duration();
        }

        // get time of arrival node
        let last_node = graph.edge_endpoints(edges[edges.len() - 1]).unwrap().1; // todo .1 or .0 depends on final implementation
        let real_arrival_time = graph[last_node].time();

        // calculate delay between planned and real_arrival
        let travel_delay = real_arrival_time as i64 - planned_arrival_time as i64;

        Self {
            travel_cost,
            travel_duration: duration,
            utilization,
            travel_delay,
            edges: edges.into_iter().collect(),
        }
    }

    /// returns cost of this path
    pub fn cost(&self) -> i64 {
        self.travel_cost as i64 + self.travel_delay
    }

    pub fn travel_cost(&self) -> u64 {
        self.travel_cost
    }

    pub fn duration(&self) -> u64 {
        self.travel_duration
    }

    pub fn utilization(&self) -> u64 {
        self.utilization
    }

    pub fn travel_delay(&self) -> i64 {
        self.travel_delay
    }

    pub fn intersecting_edges(&self, other: &Self) -> Vec<EdgeIndex> {
        self.edges.intersection(&other.edges).cloned().collect()
    }

    /// print this path as human readable traval plan
    pub fn to_human_readable_string(
        &self,
        graph: &DiGraph<TimetableNode, TimetableEdge>,
    ) -> String {
        let mut result = String::new();

        for edge in self.edges.iter() {
            let edge_weight = &graph[*edge];

            if edge_weight.is_trip() || edge_weight.is_walk() {
                // only show edges for trips and walks

                let (source_node, target_node) = graph.edge_endpoints(*edge).unwrap();

                let source_node_string = graph[source_node].station_name();
                let target_node_string = graph[target_node].station_name();

                result = format!(
                    "{}\n{} -> {} -> {}",
                    result,
                    source_node_string,
                    edge_weight.kind_as_str(),
                    target_node_string
                );
            }
        }

        result
    }

    /// format path to a reduced readable consecutive sequence of Arrival/Departure nodes and Trip/Walk edges  
    pub fn to_location_time_and_type(
        &self,
        graph: &DiGraph<TimetableNode, TimetableEdge>,
    ) -> Vec<(String, u64, String)> {
        // For arrival nodes or departure nodes save the following: (station, time, kind) with kind=Arrival or kind=Departure
        // For walk edges save ("", duration, Walk)
        // For trip edges save (trip_id, duration, Trip)
        let mut travel = Vec::new();

        // start with the first node if arrival (or departure, what should not be the case)
        let (node_a_index, _) = graph.edge_endpoints(self.edges[0]).unwrap();
        let node_a = &graph[node_a_index];
        if node_a.is_arrival() || node_a.is_departure() {
            travel.push((
                node_a.station_name(),
                node_a.time(),
                node_a.kind_as_str().to_string(),
            ));
        }

        // summed trip duration for consecutive trip edges
        let mut trip_duration = 0;

        // the trip id if currently in trip
        let mut current_trip = 0;

        for edge_index in &self.edges {
            let edge = &graph[*edge_index];
            let (node_a_index, node_b_index) = graph.edge_endpoints(*edge_index).unwrap();
            let node_a = &graph[node_a_index];
            let node_b = &graph[node_b_index];

            // if edge is ride or waiting_in_train -> currently in trip -> add duration and store current trip id
            if edge.is_trip() || edge.is_wait_in_train() {
                trip_duration += edge.duration();
                current_trip = node_a.trip_id().unwrap();
            } else {
                // if edge is not ride or wait in train and duration != 0 -> last edge before was trip edge -> save trip and node_a in travel
                if trip_duration != 0 {
                    travel.push((current_trip.to_string(), trip_duration, "Trip".to_string()));
                    travel.push((
                        node_a.station_name(),
                        node_a.time(),
                        node_a.kind_as_str().to_string(),
                    ));
                    trip_duration = 0;
                    current_trip = 0;
                }

                // if edge is walk
                if edge.is_walk() {
                    travel.push((
                        "".to_string(),
                        node_b.time() - node_a.time(),
                        edge.kind_as_str().to_string(),
                    ));
                }

                // if node_b is arrival (after walk) or node_b is departure
                if node_b.is_arrival() || node_b.is_departure() {
                    travel.push((
                        node_b.station_name(),
                        node_b.time(),
                        node_b.kind_as_str().to_string(),
                    ));
                }
            }
        }

        travel
    }

    pub fn display(&self, graph: &DiGraph<TimetableNode, TimetableEdge>) {
        for (location, time, kind) in self.to_location_time_and_type(graph) {
            if kind == "Arrival" || kind == "Departure" {
                println!("{} at station {}, time={} ->", kind, location, time,)
            } else {
                let mut in_trip = "".to_string();
                if location != "" {
                    in_trip = format!(" in trip {}", location);
                }

                println!("{} with duration {}{} ->", kind, time, in_trip)
            }
        }
    }

    pub fn to_string(&self, graph: &DiGraph<TimetableNode, TimetableEdge>) -> String {
        let mut path_string = String::new();
        for (location, time, kind) in self.to_location_time_and_type(graph) {
            path_string += &format!("{}.{}.{}->", location, time, kind);
        }
        path_string.pop();
        path_string.pop();
        path_string
    }

    /// builds subgraph that only contains nodes connected by edges
    pub fn create_subgraph_from_edges(
        &self,
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        filepath: &str,
    ) {
        let new_graph = graph.filter_map(
            |node_index, node_weight| {
                // check if at least one incoming/outgoing edge of current node is in HashSet of edges
                let mut walker = graph.neighbors_undirected(node_index).detach();
                while let Some((current_edge, _)) = walker.next(graph) {
                    if self.edges.contains(&current_edge) {
                        return Some(node_weight.clone());
                    }
                }

                // no edge in set -> do not include node in graph
                None
            },
            |edge_index, edge_weight| {
                if self.edges.contains(&edge_index) {
                    Some(edge_weight.clone())
                } else {
                    None
                }
            },
        );

        let dot_code = format!("{:?}", Dot::with_config(&new_graph, &[]));

        BufWriter::new(
            File::create(filepath).expect(&format!("Could not create dot-file at {}", filepath)),
        )
        .write(dot_code.as_bytes())
        .unwrap();
    }

    // /// returns a Vec<(missing capacity, edge)> that do not have enough capacity left for this path
    // /// if Vec empty -> all edges fit
    // pub fn colliding_edges(
    //     &self,
    //     graph: &DiGraph<TimetableNode, TimetableEdge>,
    // ) -> Vec<(u64, EdgeIndex)> {
    //     let mut colliding = Vec::new();

    //     for edge_index in self.edges.iter() {
    //         let remaining_capacity = graph[*edge_index].get_remaining_capacity();
    //         if remaining_capacity < self.utilization {
    //             colliding.push((self.utilization - remaining_capacity, *edge_index));
    //         }
    //     }

    //     colliding
    // }

    /// occupy self to graph (add utilization to edges)
    #[inline]
    pub fn strain_to_graph(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        strained_edges: &mut HashSet<EdgeIndex>,
    ) {
        for edge in self.edges.iter() {
            graph[*edge].increase_utilization(self.utilization);

            // also add edge to set of strained edges
            strained_edges.insert(*edge);
        }
    }

    /// release self from graph (remove utilization from edges)
    #[inline]
    pub fn relieve_from_graph(
        &self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        strained_edges: &mut HashSet<EdgeIndex>,
    ) {
        for edge in self.edges.iter() {
            let timetable_edge = &mut graph[*edge];
            timetable_edge.decrease_utilization(self.utilization);

            if timetable_edge.utilization() == 0 {
                // utilization is zero (edge is not strained) -> remove from strained_edges
                strained_edges.remove(edge);
            }
        }
    }

    // /// get index of path with minimal cost from a list of paths
    // pub fn get_best_path(paths: &Vec<Self>) -> Option<usize> {
    //     let mut score = 0;
    //     let mut index = None;
    //     for (i, path) in paths.iter().enumerate() {
    //         match index {
    //             Some(j) => {
    //                 if path.score() < score {
    //                     score = path.score();
    //                     index = Some(i)
    //                 }
    //             }
    //             None => {
    //                 score = path.score();
    //                 index = Some(i)
    //             }
    //         }
    //     }
    //     index
    // }

    /// iterative deeping depth-first-search (IDDFS)
    pub fn all_paths_iddfs(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        start: NodeIndex,
        destination_station_id: u64, // condition that determines whether goal node was found

        max_duration: u64,
        budgets: &[u64], // maximum number of transfers to follow
    ) -> Vec<Vec<EdgeIndex>> {
        for budget in budgets {
            print!("budget={} ... ", budget);
            io::stdout().flush().unwrap();

            let edge_sets = Self::recursive_dfs_search(
                graph,
                start,
                destination_station_id,
                max_duration,
                *budget,
            );

            if edge_sets.len() > 0 {
                // found at least one path -> return
                return edge_sets;
            }
        }

        Vec::new()
    }

    // launcher of recursive implementation of dfs
    // returns a vec of paths along with their remaining_duration
    pub fn recursive_dfs_search(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        start: NodeIndex,
        destination_station_id: u64,

        max_duration: u64,
        budget: u64, // initial search budget (each edge has cost that needs to be payed)
    ) -> Vec<Vec<EdgeIndex>> {
        // println!("all_paths_dfs(from={:?}, to={:?}, min_capacity={}, max_duration={})", from, to, min_capacity, max_duration);

        let mut results = Vec::new();
        let mut edge_stack = Vec::new();

        // use this hashmap to track at which time the station's transfer was already visited (only replace with earlier times)
        // station_id -> time
        let mut visited_stations: HashMap<u64, u64> = HashMap::with_capacity(10000000);

        let mut counter_already_visited_earlier = 0;
        let mut counter_out_of_calls = 0;
        let mut counter_out_of_duration = 0;
        let mut counter_out_of_budget = 0;

        Self::recursive_dfs_search_helper(
            graph,
            &mut results,
            start,
            destination_station_id,
            &mut edge_stack,
            &mut visited_stations,
            max_duration,
            budget,
            &mut counter_already_visited_earlier,
            &mut counter_out_of_calls,
            &mut counter_out_of_duration,
            &mut counter_out_of_budget,
        );

        print!(
            "[ave={} ooc={} ood={} oob={}] ",
            counter_already_visited_earlier,
            counter_out_of_calls,
            counter_out_of_duration,
            counter_out_of_budget
        );

        results
    }

    fn recursive_dfs_search_helper(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        results: &mut Vec<Vec<EdgeIndex>>, // paths found until now
        current_node: NodeIndex,
        destination_station_id: u64,
        edge_stack: &mut Vec<EdgeIndex>, // visited edges (in order of visit)

        // recursion anchors (if zero)
        visited_stations: &mut HashMap<u64, u64>,
        remaining_duration: u64,
        remaining_budget: u64,

        counter_already_visited_earlier: &mut u64,
        counter_out_of_calls: &mut u64,
        counter_out_of_duration: &mut u64,
        counter_out_of_budget: &mut u64,
    ) {
        let current_station_weight = &graph[current_node];
        let current_station_weight_id = current_station_weight.station_id();
        let current_station_weight_time = current_station_weight.time();

        if current_station_weight_id == destination_station_id {
            // found destination node -> don't further continue this path
            results.push(edge_stack.clone());
        } else {
            if current_station_weight.is_transfer() {
                // check we visited current station's transfer at an earlier point already
                // we would then stop following current path
                match visited_stations.get(&current_station_weight_id) {
                    Some(last_station_time) => {
                        // we visited this station before
                        // now check if we visited it earlier

                        if *last_station_time <= current_station_weight_time {
                            // we visited this station earlier
                            *counter_already_visited_earlier += 1;
                            return;
                        } else {
                            // we found an earlier visit -> replace time an continue search
                            visited_stations.insert(current_station_weight_id, current_station_weight_time);
                        }
                    }
                    None => {
                        // we did not visit this station before -> insert current visit time and continue search
                        visited_stations.insert(current_station_weight_id, current_station_weight_time);
                    }
                }
            }

            let mut walker = graph.neighbors(current_node).detach();

            // iterate over all outgoing edges
            while let Some((next_edge, next_node)) = walker.next(graph) {
                // lookup edge's cost
                let next_edge_weight = &graph[next_edge];
                let next_edge_weight_cost = next_edge_weight.travel_cost();
                let next_edge_weight_duration = next_edge_weight.duration();

                // if edge_stack.len() == 80 {
                //     *counter_out_of_calls += 1;
                //     return;
                // }

                if next_edge_weight_cost > remaining_budget {
                    *counter_out_of_budget += 1;
                    return;
                }

                // if edge_weight_duration > remaining_duration {
                //     *counter_out_of_duration += 1;
                //     return;
                // }

                // -> we can "afford" going using next_edge

                // add next_edge to stack
                edge_stack.push(next_edge);

                // make recursive call
                &mut Self::recursive_dfs_search_helper(
                    graph,
                    results,
                    next_node,
                    destination_station_id,
                    edge_stack,
                    visited_stations,
                    remaining_duration - next_edge_weight_duration,
                    remaining_budget - next_edge_weight_cost,
                    counter_already_visited_earlier,
                    counter_out_of_calls,
                    counter_out_of_duration,
                    counter_out_of_budget,
                );

                // remove next_edge from stack
                edge_stack.pop();
            }
        }
    }

    /// petgraph native depth first search (using visitors)
    /// currently fastest implementation (full traversation, no duration/budget/capacity limitation)
    pub fn dfs_visitor_search(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        start: NodeIndex,
        destination_station_id: u64, // condition that determines whether goal node was found

        utilization: u64, // number of passengers, weight of load, etc.
        planned_arrival: u64,

        limit_paths: usize,
    ) -> Vec<Self> {
        let mut paths = Vec::new();

        let mut predecessor = vec![NodeIndex::end(); graph.node_count()];

        let start_time = graph[start].time();

        depth_first_search(graph, Some(start), |event| {
            if let DfsEvent::TreeEdge(u, v) = event {
                predecessor[v.index()] = u;

                let timetable_node = &graph[v];
                if timetable_node.time() - start_time > 4 * (planned_arrival - start_time) + 60 {
                    return Control::Prune;
                }

                if graph[v].station_id() == destination_station_id {
                    // we found destination node -> use predecessor map to look-up edge path
                    // start at destination node (to) and "walk" back to start (from), collect all nodes in path vec and then reverse vec

                    let mut next = v; //destination_station_id;
                    let mut node_path = vec![next];

                    while next != start {
                        let pred = predecessor[next.index()];
                        node_path.push(pred);
                        next = pred;
                    }
                    node_path.reverse();

                    // found_destinations.push(to.clone());
                    let mut edges = Vec::new();

                    for transfer_slice in node_path.windows(2) {
                        // iterate over all pairs of nodes in node_path

                        // add index of edge between node pair to edges
                        edges.push(
                            graph
                                .find_edge(transfer_slice[0], transfer_slice[1])
                                .unwrap(),
                        );
                    }

                    // create and insert Self
                    paths.push(Self::new(graph, edges, utilization, planned_arrival));

                    if limit_paths != 0 && paths.len() >= limit_paths {
                        return Control::Break(v);
                    }
                    return Control::Prune;
                }
            }

            // always continue dfs
            Control::<NodeIndex>::Continue
        });

        paths
    }
}

/// working too good
fn all_simple_paths_dfs_dorian(
    graph: &'static DiGraph<TimetableNode, TimetableEdge>,
    start: NodeIndex,
    destination: NodeIndex,
    max_duration: u64,
    max_rides: u64,
) -> impl Iterator<Item = (u64, Vec<EdgeIndex>)> {
    //Vec<(u64, Vec<EdgeIndex>)> {

    // list of already visited nodes
    let mut visited: IndexSet<EdgeIndex> = IndexSet::new();

    // list of childs of currently exploring path nodes,
    // last elem is list of childs of last visited node
    let mut stack = vec![graph.neighbors_directed(start, Outgoing).detach()];
    let mut durations: Vec<u64> = Vec::new();
    let mut rides: Vec<u64> = vec![0];

    //let mut bfs = self.graph.neighbors_directed(from_node_index, Outgoing).detach();
    //let mut a = bfs.next(&self.graph);
    let path_finder = from_fn(move || {
        while let Some(children) = stack.last_mut() {
            if let Some((child_edge_index, child_node_index)) = children.next(graph) {
                let mut duration = graph.edge_weight(child_edge_index).unwrap().duration();
                if durations.iter().sum::<u64>() + duration < max_duration
                    && rides.iter().sum::<u64>() < max_rides
                {
                    if child_node_index == destination {
                        let path = visited
                            .iter()
                            .cloned()
                            .chain(Some(child_edge_index))
                            .collect::<Vec<EdgeIndex>>();
                        return Some((
                            max_duration - durations.iter().sum::<u64>() - duration,
                            path,
                        ));
                    } else if !visited.contains(&child_edge_index) {
                        let edge_weight = graph.edge_weight(child_edge_index).unwrap();
                        durations.push(edge_weight.duration());
                        // only count ride to station and walk to station as limit factor
                        // if edge_weight.is_ride_to_station() || edge_weight.is_walk_to_station() {
                        //     rides.push(1);
                        // } else {
                        //     rides.push(0);
                        // };
                        //rides.push(1);
                        visited.insert(child_edge_index);
                        stack.push(
                            graph
                                .neighbors_directed(child_node_index, Outgoing)
                                .detach(),
                        );
                    }
                } else {
                    let mut children_any_to_node_index = false;
                    let mut edge_index = None;
                    let mut children_cloned = children.clone();
                    while let Some((c_edge_index, c_node_index)) = children_cloned.next(graph) {
                        if c_node_index == destination {
                            children_any_to_node_index = true;
                            edge_index = Some(c_edge_index);
                            duration = graph.edge_weight(child_edge_index).unwrap().duration();
                            break;
                        }
                    }
                    if (child_node_index == destination || children_any_to_node_index)
                        && (durations.iter().sum::<u64>() + duration >= max_duration
                            || rides.iter().sum::<u64>() >= max_rides)
                    {
                        let path = visited
                            .iter()
                            .cloned()
                            .chain(edge_index)
                            .collect::<Vec<EdgeIndex>>();
                        return Some((0, path));
                    }
                    stack.pop();
                    visited.pop();
                    durations.pop();
                    rides.pop();
                }
            } else {
                stack.pop();
                visited.pop();
                durations.pop();
                rides.pop();
            }
        }
        None
    });

    //path_finder.collect::<Vec<_>>()
    path_finder
}

// pub enum Object {
//     Edge(EdgeWeight),
//     Node(NodeWeight)
// }

// pub enum ObjectIndex {
//     EdgeIndex(EdgeIndex),
//     NodeIndex(NodeIndex),
// }

// // creates a subgraph of self with only the part of the graph of specified paths
// pub fn create_subgraph_with_nodes_old(graph: &mut DiGraph<NodeWeight, EdgeWeight>, paths: Vec<Path>, node_index_graph_subgraph_mapping: &mut HashMap<NodeIndex, NodeIndex>) -> Vec<Vec<ObjectIndex>> {

//     let mut subgraph = DiGraph::new();

//     //let mut subgraph = DiGraph::new();
//     let mut subgraph_paths: Vec<Vec<ObjectIndex>> = Vec::new();

//     // iterate all paths in graph
//     for path in paths {

//         let mut subgraph_path_indices: Vec<ObjectIndex> = Vec::new();
//         let mut path_max_flow: u64 = std::u64::MAX;
//         let mut path_edge_indices: Vec<EdgeIndex> = Vec::new();

//         // iterate over all NodeIndex pairs in this path
//         for graph_node_index_pair in path.windows(2) {

//             // check if the first node already exists in subgraph
//             let subgraph_node_a_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[0]) {
//                 Some(subgraph_node_index) => *subgraph_node_index,
//                 None => {
//                     // clone NodeWeight from graph
//                     let node_weight = graph.node_weight(graph_node_index_pair[0]).unwrap().clone();

//                     // create new node in subgraph
//                     let subgraph_node_index = subgraph.add_node(node_weight);

//                     // insert mapping into HashMap
//                     node_index_graph_subgraph_mapping.insert(graph_node_index_pair[0], subgraph_node_index.clone());

//                     subgraph_node_index
//                 }
//             };

//             // check if the second node already exists in subgraph
//             let subgraph_node_b_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[1]) {
//                 Some(subgraph_node_index) => *subgraph_node_index,
//                 None => {
//                     // clone NodeWeight from graph
//                     let node_weight = graph.node_weight(graph_node_index_pair[1]).unwrap().clone();

//                     // create new node in subgraph
//                     let subgraph_node_index = subgraph.add_node(node_weight);

//                     // insert mapping into HashMap
//                     node_index_graph_subgraph_mapping.insert(graph_node_index_pair[1], subgraph_node_index);

//                     subgraph_node_index
//                 }
//             };

//             // add outgoing node to path if path is empty
//             if subgraph_path_indices.is_empty() {
//                 subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_a_index));
//             };

//             // create edge if there was created at least one new node
//             let subgraph_edge_weight = match subgraph.find_edge(subgraph_node_a_index, subgraph_node_b_index) {
//                 Some(subgraph_edge_index) => {
//                     // add edge to path
//                     subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
//                     path_edge_indices.push(subgraph_edge_index);
//                     subgraph.edge_weight(subgraph_edge_index).unwrap()
//                 },
//                 None => {
//                     let graph_edge_index = graph.find_edge(graph_node_index_pair[0], graph_node_index_pair[1]).unwrap();
//                     let subgraph_edge_weight = graph.edge_weight(graph_edge_index).unwrap().clone();

//                     let subgraph_edge_index = subgraph.add_edge(subgraph_node_a_index, subgraph_node_b_index, subgraph_edge_weight);
//                     // add edge to path
//                     subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
//                     path_edge_indices.push(subgraph_edge_index);
//                     subgraph.edge_weight(subgraph_edge_index).unwrap()
//                 }
//             };

//             // update max_flow if edge capacity is smaller current path_max_flow
//             let edge_remaining_flow = subgraph_edge_weight.get_capacity() - subgraph_edge_weight.get_utilization();
//             if edge_remaining_flow < path_max_flow {
//                 path_max_flow = edge_remaining_flow;
//             };

//             subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_b_index));
//         };

//         subgraph_paths.push(subgraph_path_indices);

//         // set utilization to all edges of path
//         for path_edge_index in path_edge_indices {
//             subgraph.edge_weight_mut(path_edge_index).unwrap().increase_utilization(path_max_flow);
//             //println!("{}, {}", path_max_flow, subgraph.edge_weight(path_edge_index).unwrap().get_utilization())
//         }
//     }

//     subgraph_paths
// }

// // currently not working (problems with last ancestor path element on stack (only first child works, siblings will have the same path))
// pub fn all_paths_dfs_iterative(
//     graph: &DiGraph<NodeWeight, EdgeWeight>,
//     from: NodeIndex,
//     to: NodeIndex, // condition that determines whether goal node was found

//     min_capacity: u64,
//     max_duration: u64,
//     max_budget: u64,
// ) -> Vec<Vec<EdgeIndex>> {
//     // list of all found paths
//     let mut paths = Vec::new();

//     // maps every seen NodeIndex to its parent (NodeIndex, EdgeIndex)
//     let mut parent: HashMap<NodeIndex, (NodeIndex, EdgeIndex)> = HashMap::with_capacity(graph.node_count());

//     // (<nodes>, remaining duration, remaining budget)
//     let mut to_visit: Vec<(NodeIndex, u64, u64)> = vec![(from, max_duration, max_budget)];

//     while let Some((current, remaining_duration, remaining_budget)) = to_visit.pop() {

//         if current == to {

//             let mut path: Vec<EdgeIndex> = Vec::new();

//             // collect all EdgeIndex until root
//             let mut current = current;
//             while let Some((parent_node, parent_edge)) = parent.get(&current) {
//                 path.push(*parent_edge);
//                 current = *parent_node;
//             }

//             path.reverse();
//             paths.push(path);

//         } else {

//             // iterate over all outgoing edges
//             let mut walker = graph.neighbors_directed(current, Outgoing).detach();
//             while let Some((next_edge, next_node)) = walker.next(graph) {

//                 let edge_weight = &graph[next_edge];
//                 let edge_weight_duration = edge_weight.get_duration();
//                 let edge_weight_cost = edge_weight.get_cost();

//                 if edge_weight_duration <= remaining_duration && edge_weight.get_remaining_capacity() >= min_capacity && edge_weight_cost <= remaining_budget {

//                     // add parent edge and node of current
//                     match parent.insert(next_node, (current, next_edge)) {
//                         Some(_) => {},
//                         None => {}
//                     };

//                     to_visit.push((
//                         next_node,
//                         remaining_duration - edge_weight_duration,
//                         remaining_budget - edge_weight_cost,
//                     ));
//                 }
//             }
//         }
//     }

//     paths
// }
