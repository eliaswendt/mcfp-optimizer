use indexmap::IndexSet;
use petgraph::{EdgeDirection::Outgoing, graph::{DiGraph, EdgeIndex, NodeIndex}, visit::{Control, DfsEvent, depth_first_search}};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet, iter::from_fn};

use super::{TimetableEdge, TimetableNode};

#[derive(Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Path {
    pub travel_cost: u64,    // cost for this path
    duration: u64,    // duration of this path
    utilization: u64, // number of passengers

    pub edges: IndexSet<EdgeIndex>,
}

impl Path {

    pub fn utilization_cost(&self, graph: &DiGraph<TimetableNode, TimetableEdge>) -> u64 {
        self.edges.iter().map(|e| graph[*e].utilization_cost()).sum()
    }


    #[inline]
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    #[inline]
    pub fn duration(&self) -> u64 {
        self.duration
    }

    pub fn intersecting_edges(&self, other: &Self) -> Vec<EdgeIndex> {
        self.edges.intersection(&other.edges).cloned().collect()
    }

    /// calculates a score that will be useful for heuristic search
    #[inline]
    pub fn score(&self) -> u64 {
        (self.travel_cost + self.duration) / 2
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
    pub fn strain(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) {
        for edge_index in self.edges.iter() {
            graph[*edge_index].increase_utilization(self.utilization)
        }
    }

    /// release self from graph (remove utilization from edges)
    pub fn relieve(&self, graph: &mut DiGraph<TimetableNode, TimetableEdge>) {
        for edge_index in self.edges.iter() {
            graph[*edge_index].decrease_utilization(self.utilization)
        }
    }

    /// get index of path with minimal cost from a list of paths
    pub fn get_best_path(paths: &Vec<Self>) -> Option<usize> {
        let mut score = 0;
        let mut index = None;
        for (i, path) in paths.iter().enumerate() {
            match index {
                Some(j) => {
                    if path.score() < score {
                        score = path.score();
                        index = Some(i)
                    }
                }
                None => {
                    score = path.score();
                    index = Some(i)
                }
            }
        }
        index
    }

    /// iterative deeping depth-first-search (IDDFS)
    pub fn all_paths_iddfs(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        from: NodeIndex,
        to: NodeIndex, // condition that determines whether goal node was found

        min_capacity: u64,
        max_duration: u64,

        budget_steps: &[u64] // maximum number of transfers to follow
    ) -> Vec<Self> {

        for budget in budget_steps {

            print!("budget={} ... ", budget);

            let paths = Self::search_recursive_dfs(
                graph,
                from,
                to,
                min_capacity,
                max_duration,
                *budget,
            );

            if paths.len() > 0 {
                // found at least one path -> return
                return paths;
            }
        }

        print!("no limit ... ");

        let mut predecessor = vec![NodeIndex::end(); graph.node_count()];
        depth_first_search(graph, Some(from), | event| {
            if let DfsEvent::TreeEdge(u, v) = event {
                predecessor[v.index()] = u;
                if v == to {
                    return Control::Break(v);
                }
            }
            Control::Continue
        });
        
        let mut next = to;
        let mut path = vec![next];
        while next != from {
            let pred = predecessor[next.index()];
            path.push(pred);
            next = pred;
        }
        path.reverse();
        //println!("{:?}", path);
        //let mut remaining_duration = 0;
        //let mut remaining_budget = 0;
        let mut edges = IndexSet::new();

        for transfer_slice in path.windows(2) {
            let edge_index = graph.find_edge(transfer_slice[0], transfer_slice[1]);
            edges.insert(edge_index.unwrap());
        }

        vec![Self {
            travel_cost: 0,
            duration: 0,

            utilization: min_capacity,

            edges: edges,
        }]

    }

    // launcher of recursive implementation of dfs
    // returns a vec of paths along with their remaining_duration
    pub fn search_recursive_dfs(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        from: NodeIndex,
        to: NodeIndex, // condition that determines whether goal node was found
        
        utilization: u64, // number of passengers, weight of load, etc.
        max_duration: u64,
        max_budget: u64, // initial search budget (each edge has cost that needs to be payed)
    ) -> Vec<Self> {
        // println!("all_paths_dfs(from={:?}, to={:?}, min_capacity={}, max_duration={})", from, to, min_capacity, max_duration);

        let mut paths = Vec::new();
        let mut visited = IndexSet::new();

        Self::search_recursive_dfs_helper(
            graph,
            &mut paths,
            from,
            to,
            &mut visited,
            utilization,
            max_duration,
            max_budget,
        );

        paths
            .into_iter()
            .map(|(remaining_duration, remaining_budget, edges)| Self {
                travel_cost: max_budget - remaining_budget,
                duration: max_duration - remaining_duration,

                utilization,

                edges: edges.into_iter().collect(),
            })
            .collect()
    }

    fn search_recursive_dfs_helper(
        graph: &DiGraph<TimetableNode, TimetableEdge>,
        paths: &mut Vec<(u64, u64, IndexSet<EdgeIndex>)>, // paths found until now
        current: NodeIndex,
        to: NodeIndex,
        visited: &mut IndexSet<EdgeIndex>, // vec of visited edges (in order of visit)

        // recursion anchors (if zero)
        min_capacity: u64,
        remaining_duration: u64,
        remaining_budget: u64,
    ) {
        // println!("all_paths_dfs_recursive(current={:?}, goal={:?}, visited.len()={}, min_capacity={}, remaining_duration={})", current, to, visited.len(), min_capacity, remaining_duration);
        // println!("remaining_duration: {}", remaining_duration);

        if current == to {
            // take all edge indices (in order of visit) and insert them into a vec
            paths.push((remaining_duration, remaining_budget, visited.clone()));
        } else {
            let mut walker = graph.neighbors_directed(current, Outgoing).detach();

            // iterate over all outgoing edges
            while let Some((next_edge, next_node)) = walker.next(graph) {
                let edge_weight = &graph[next_edge];
                let edge_weight_duration = edge_weight.duration();
                let edge_weight_cost = edge_weight.travel_cost();

                if edge_weight_duration <= remaining_duration
                    && edge_weight.capacity_hard_limit() >= min_capacity
                    && edge_weight_cost <= remaining_budget
                {
                    // edge can handle the minium required capacity and does not take longer then the remaining duration

                    // add next_edge for next call
                    visited.insert(next_edge);

                    &mut Self::search_recursive_dfs_helper(
                        graph,
                        paths,
                        next_node,
                        to,
                        visited,
                        min_capacity,
                        remaining_duration - edge_weight_duration,
                        remaining_budget - edge_weight_cost,
                    );

                    // remove next_edge from visited
                    visited.pop();
                }
            }
        }
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score().cmp(&other.score())
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.score() == other.score()
    }
}

/// builds subgraph that only contains nodes connected by edges
pub fn create_subgraph_from_edges(
    graph: &DiGraph<TimetableNode, TimetableEdge>,
    edges: &HashSet<EdgeIndex>,
) -> DiGraph<TimetableNode, TimetableEdge> {
    graph.filter_map(
        |node_index, node_weight| {
            // check if at least one incoming/outgoing edge of current node is in HashSet of edges
            let mut walker = graph.neighbors_undirected(node_index).detach();
            while let Some((current_edge, _)) = walker.next(graph) {
                if edges.contains(&current_edge) {
                    return Some(node_weight.clone());
                }
            }

            // no edge in set -> do not include node in graph
            None
        },
        |edge_index, edge_weight| {
            if edges.contains(&edge_index) {
                Some(edge_weight.clone())
            } else {
                None
            }
        },
    )
}

/// not working
fn all_simple_paths_dfs_dorian(
    graph: &'static DiGraph<TimetableNode, TimetableEdge>,
    from_node_index: NodeIndex,
    to_node_index: NodeIndex,
    max_duration: u64,
    max_rides: u64,
) -> impl Iterator<Item = (u64, Vec<EdgeIndex>)> {
    //Vec<(u64, Vec<EdgeIndex>)> {

    // list of already visited nodes
    let mut visited: IndexSet<EdgeIndex> = IndexSet::new();

    // list of childs of currently exploring path nodes,
    // last elem is list of childs of last visited node
    let mut stack = vec![graph.neighbors_directed(from_node_index, Outgoing).detach()];
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
                    if child_node_index == to_node_index {
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
                        if c_node_index == to_node_index {
                            children_any_to_node_index = true;
                            edge_index = Some(c_edge_index);
                            duration = graph.edge_weight(child_edge_index).unwrap().duration();
                            break;
                        }
                    }
                    if (child_node_index == to_node_index || children_any_to_node_index)
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
