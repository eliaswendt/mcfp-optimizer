use std::collections::HashMap;

use petgraph::graph::{DiGraph, EdgeIndex};

use rand::Rng;

use crate::model::{
    group::Group,
    path::{self},
    TimetableEdge, TimetableNode,
};

// use std::{collections::HashMap, sync::atomic::AtomicU64};

// use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

// use crate::model::{TimetableEdge, TimetableNode, group::Group, path::Path};

pub fn optimize_overloaded_graph(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
) -> HashMap<u64, usize> {
    // group_2_path_index:      mapping from group ids to selected path (identified by index in group's path vec)
    // edges_2_groups:          mapping from edge indices to a list of group tuples (index in group list, group ids) that occupy the edge
    // select a path for each group and occupy edge with group
    let (mut group_2_path_index, mut edges_2_groups) = select_path_per_group(graph, groups);

    // -------------- Step 1: Initialization --------------

    // a list of overcrowded edges
    let mut overcrowded_edges = HashMap::new();

    // get all overcrowded edges
    for (edge_index, occupying_groups) in edges_2_groups.iter() {
        let timetable_edge = graph.edge_weight(*edge_index).unwrap();
        if timetable_edge.get_utilization() > timetable_edge.get_capacity() {
            println!(
                "Overcrowded edge found: capacity={}, utilization={}, groups={:?}",
                timetable_edge.get_capacity(),
                timetable_edge.get_utilization(),
                occupying_groups
            );
            overcrowded_edges.insert(
                *edge_index,
                timetable_edge.get_utilization() - timetable_edge.get_capacity(),
            );
        }
    }

    // create random number generator
    let mut rng = rand::thread_rng();

    // step t of optimization
    let mut t = 1;
    // max steps
    let mut t_max = 1000;

    // set current best solution
    let mut best_solution = overcrowded_edges.clone();

    println!(
        "Initial solution: overcrowding_rating={}",
        rate_overcrowding(&best_solution)
    );

    // improve occupying
    while t <= t_max {
        // break if no edge is overcrowded
        if overcrowded_edges.len() == 0 {
            break;
        }

        // -------------- Step 2: Local change --------------

        let r = rng.gen_range(0..overcrowded_edges.len());
        // find one overcrowded edge that may improve the current best solution
        let overcrowded_edge = **overcrowded_edges.keys().collect::<Vec<_>>().get(r).unwrap();

        // get occupying groups of the edge
        let mut occupying_groups = edges_2_groups.get(&overcrowded_edge).unwrap().to_vec();

        //let overcrowding = timetable_edge.get_utilization() - timetable_edge.get_capacity();

        // find one group randomly
        let j = rng.gen_range(0..occupying_groups.len());
        let group_index = occupying_groups.remove(j).0;
        edges_2_groups.insert(overcrowded_edge, occupying_groups);
        let group = groups.get(group_index).unwrap();
        // relieve edges of previous path
        let path_index = group_2_path_index.get(&group.id).unwrap();
        let path = group.paths.get(*path_index).unwrap();
        path.relieve(graph);

        // remove from overcrowded edges if utilization <= capacity
        for edge_index in path.edges.iter() {
            let collected_keys = overcrowded_edges.keys().collect::<Vec<_>>();
            if collected_keys.contains(&edge_index) {
                let timetable_edge = graph[*edge_index].clone();
                if timetable_edge.get_utilization() <= timetable_edge.get_capacity() {
                    overcrowded_edges.remove(edge_index);
                }
            }
        }

        // find new path
        let next_path_index = rng.gen_range(0..group.paths.len());
        let next_path = group.paths.get(next_path_index).unwrap();

        // println!("Group: {}", group_index);
        // let mut in_all_paths = true;
        // for path in group.paths.iter() {
        //     if !path.edges.contains(&overcrowded_edge.clone()) {
        //         in_all_paths = false
        //     }
        // }

        // println!("In all paths: {} {} {:?}", in_all_paths, group_index, graph[overcrowded_edge]);
        // if !in_all_paths {
        //     break
        // }

        // strain edges of new path
        next_path.strain(graph);

        // set path as new path of group
        group_2_path_index.insert(group.id, next_path_index);

        // check for all edges if overcrowded
        for edge_index in next_path.edges.iter() {
            if !edges_2_groups.contains_key(&edge_index) {
                edges_2_groups.insert(*edge_index, vec![(group_index, group.id)]);
            } else {
                let mut edge_groups = edges_2_groups.get(&edge_index).unwrap().to_vec();
                edge_groups.push((group_index, group.id));
                edges_2_groups.insert(*edge_index, edge_groups);
            }
            let overcrowded =
                edge_overcrowding(graph, *edge_index, edges_2_groups.get(&edge_index).unwrap());
            if overcrowded > 0 {
                overcrowded_edges.insert(*edge_index, overcrowded as u64);
            }
        }

        // -------------- Step 3: Selection --------------

        // check if overloading is smaller than previous overloading
        if rate_overcrowding(&overcrowded_edges) < rate_overcrowding(&best_solution) {
            best_solution = overcrowded_edges.clone();
        } else {
            let temperature = next_temperature(t, t_max);
            let numerator = rate_overcrowding(&best_solution) as f64
                - rate_overcrowding(&overcrowded_edges) as f64;
            let probability = (numerator / temperature).exp();
            let r = rng.gen_range(0.0..1.0);
            //println!("numerator={}, temp={}, r={}, prob={}", numerator, temperature, r, probability);
            if r < probability {
                best_solution = overcrowded_edges.clone();
            } else {
                // undo changes in graph
                next_path.relieve(graph);
                path.strain(graph);
                overcrowded_edges = best_solution.clone();
            }
        }

        println!(
            "Intermediate solution: overcrowding_rating={}",
            rate_overcrowding(&best_solution)
        );

        t += 1;
    }

    println!(
        "Final solution: overcrowding_rating={}",
        rate_overcrowding(&best_solution)
    );

    group_2_path_index
}

/// rates overcrowding
pub fn rate_overcrowding(overcrowded_edges: &HashMap<EdgeIndex, u64>) -> u64 {
    let mut rating: u64 = 0;
    for (_, overcrowding) in overcrowded_edges.iter() {
        rating += overcrowding;
    }
    rating
}

/// calculates the next temperature for simulated annealing based on step t
pub fn next_temperature(t: u64, t_max: u64) -> f64 {
    let temperature = 1.0 / ((t as f64).ln());
    temperature
}

pub fn select_path_per_group(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
) -> (HashMap<u64, usize>, HashMap<EdgeIndex, Vec<(usize, u64)>>) {
    // mapping from group ids to selected path (identified by index in group's path list)
    let mut group_2_path_index: HashMap<u64, usize> = HashMap::with_capacity(groups.len());

    // mapping from edge indices to a list of group touples (index in group list, group ids) that occupy the edge
    let mut edges_2_groups: HashMap<EdgeIndex, Vec<(usize, u64)>> = HashMap::new();

    for (i, group) in groups.iter().enumerate() {
        let index = path::Path::get_best_path(&group.paths);
        match index {
            Some(index) => {
                let selected_path = group.paths.get(index).unwrap();
                selected_path.strain(graph);
                for edge_index in selected_path.edges.iter() {
                    if !edges_2_groups.contains_key(edge_index) {
                        edges_2_groups.insert(*edge_index, vec![(i, group.id)]);
                    } else {
                        let mut groups = edges_2_groups.get(edge_index).unwrap().to_vec();
                        groups.push((i, group.id));
                        edges_2_groups.insert(*edge_index, groups);
                    }
                }
                group_2_path_index.insert(group.id, index);
            }
            None => {}
        }
    }

    (group_2_path_index, edges_2_groups)
}

fn edge_overcrowding(
    graph: &DiGraph<TimetableNode, TimetableEdge>,
    edge_index: EdgeIndex,
    occupying_groups: &Vec<(usize, u64)>,
) -> i64 {
    let timetable_edge = graph.edge_weight(edge_index).unwrap();
    let overcrowding =
        timetable_edge.get_utilization() as i64 - timetable_edge.get_capacity() as i64;
    if overcrowding > 0 {
        //println!("Overcrowded edge found: edge={:?}, capacity={}, utilization={}, groups={:?}", edge_index, timetable_edge.get_capacity(), timetable_edge.get_utilization(), occupying_groups);
    }
    return overcrowding;
}

// pub fn select_path_per_group(
//     graph: &mut DiGraph<NodeWeight, EdgeWeight>,
//     groups_paths: & Vec<(u64, Vec<Path>)>,
// ) -> Vec<(u64, Path)> {
//     let mut groups_paths_sorted = groups_paths.to_vec();
//     groups_paths_sorted.sort_unstable_by_key(|(_, paths)| paths.len());
//     //println!("{:?}", groups_paths_sorted);
//     recursive_select_path_per_group(graph, &mut Vec::new(), &mut groups_paths_sorted, 0)
// }

// pub fn recursive_select_path_per_group(
//     graph: &mut DiGraph<NodeWeight, EdgeWeight>,
//     selected_paths: &mut Vec<(u64, Path)>,
//     groups_paths: &mut Vec<(u64, Vec<Path>)>,
//     pos: usize
// ) -> Vec<(u64, Path)> {
//     if pos == groups_paths.len() {
//         //println!("{}", pos);
//         return selected_paths.to_vec()
//     } else {
//         //println!("{}, {}", pos, groups_paths.len());
//         let mut best_selected_path: Vec<(u64, Path)> = Vec::with_capacity(groups_paths.len());
//         for path in groups_paths.get(pos).unwrap().1.to_vec() {
//             //println!("{}", groups_paths.get(pos).unwrap().0);
//             if path.fits(graph) {
//                 path.strain(graph);
//                 selected_paths.push((groups_paths.get(pos).unwrap().0, path.clone()));
//                 let new_path = recursive_select_path_per_group(
//                     graph,
//                     selected_paths,
//                     groups_paths,
//                     pos + 1
//                 );
//                 selected_paths.pop();
//                 path.relieve(graph);
//                 if new_path.len() > best_selected_path.len() {
//                     best_selected_path = new_path;
//                 } else if new_path.len() == best_selected_path.len() {
//                     continue; // todo: check which variant has more passengers
//                 }
//             }
//         }
//         return best_selected_path
//     }
// }

// pub fn recursive_path_straining(
//     graph: &mut DiGraph<NodeWeight, EdgeWeight>,
//     groups_paths: &[(u64, Vec<Path>)],
//     inserted_groups: &mut Vec<u64>
// ) {
//     if groups_paths.len() == 0 {
//         // recursion anchor
//         println!("inserted_groups.len()={}", inserted_groups.len());
//     } else {
//         println!("remaining_depth={}", groups_paths.len());

// // /// try to split main path set into many tiny disjunct subsets
// // pub fn calculate_intersection_sets(tagged_paths: &Vec<(u64, Path)>) {

// //     // create new graph for efficiently storing intersection relations between many paths
// //     let mut intersection_graph: DiGraph<OptimizationNode, ()> = DiGraph::new();

// //     // hashmap for efficient NodeIndex lookup (use collection of intersectiong edge indices as key)
// //     let mut intersection_graph_nodes: HashMap<Vec<EdgeIndex>, NodeIndex> = HashMap::new();

// //     // hashmap with a vec of intersection edges as key and the tagged_paths as values sharing this intersection
// //     let mut intersection_sets = HashMap::new();

// //     // iterate over all tagged_paths
// //     for (cursor, (current_group_id, current_path)) in tagged_paths.iter().enumerate() {
// //         println!("comparing tagged_path {}/{} to all subsequent ones", cursor, tagged_paths.len());

// //         let current_path_node = intersection_graph.add_node(OptimizationNode::Path {
// //             group_id: *current_group_id,
// //             path: current_path.clone()
// //         });

// //         // and comapre it to itself and all remaining tagged_paths in the vector (start is after current cursor position)
// //         for (other_group_id, other_path) in &tagged_paths[cursor+1..] {

// //             let mut intersection = current_path.intersection(other_path);
// //             intersection.sort_unstable(); // important because different ordering derives completely other key in HashMap

// //             // if not empty add intersection to hashmap
// //             if !intersection.is_empty() {
// //                 let node = match intersection_graph_nodes.get(&intersection) {
// //                     Some(node) => node,
// //                     None => {
// //                         let node = intersection_graph_nodes.insert(
// //                             intersection.clone(),
// //                             intersection_graph.add_node(OptimizationNode::Intersection)
// //                         );

// //                         // now also connect current_path_node to intersection_node

// //                     }
// //                 }

// //                 let intersection_node = intersection_graph_nodes
// //                     .entry(intersection.clone())
// //                     .or_insert(intersection_graph.add_node(OptimizationNode::Intersection {
// //                         edges: intersection
// //                     }));

// //             }
// //         }
// //     }
// // }

// /// abstraction of a solution for our problem
// struct Solution {
//     groups: Vec<Option<Path>>, // group.id as index, path as element
//     score: u64 //
// }
// impl Solution {
//     pub fn new(len: usize) -> Self {
//         Self {
//             groups: vec![None; len],
//             score: 0
//         }
//     }

//     pub fn score(&self) -> u64 {
//         self.groups.iter().map(|e| match e {
//             Some(path) => path.score(),
//             None => 0
//         }).sum()
//     }

//     /// returns all group indices who's path contains at least one of the following
//     pub fn intersecting_groups(path: &Path) -> Vec<usize> {

//     }

//     pub fn simulate_insert(group_index: usize, path: Path) -> {

//     }
// }

// // returns Vec<group_id, path> as solution
// pub fn simulated_annealing(
//     graph: &mut DiGraph<TimetableNode, TimetableEdge>,
//     groups: Vec<&Group>,
// ) -> Solution {

//     let mut solution = Solution::new(groups.len());

//     solution
// }

// pub fn recursive_path_straining(
//     graph: &mut DiGraph<TimetableNode, TimetableEdge>,
//     groups_paths: &[(u64, Vec<Path>)],
//     inserted_groups: &mut Vec<u64>,
//     n_solutions: &u128,
//     n_solutions_tried: &mut u128,
// ) {
//     if groups_paths.len() == 0 {
//         // recursion anchor
//         println!("inserted_groups.len()={}", inserted_groups.len());
//         *n_solutions_tried += 1;
//     } else {
//         // println!("remaining_depth={}", groups_paths.len());

//         let (group_id, group_paths) = groups_paths.first().unwrap();

//         // push group.id onto the stack
//         inserted_groups.push(*group_id);

//         // make recursion call with each path known for this group
//         for path in group_paths.iter() {
//             if path.fits(graph) {
//                 path.strain(graph);

//                 recursive_path_straining(graph, &groups_paths[1..], inserted_groups, n_solutions, n_solutions_tried);

//                 path.relieve(graph);
//             }
//         }

//         // remove group.id from the stack again
//         inserted_groups.pop();

//         // also try to leave-out this path by making a single recursive call without demanding
//         recursive_path_straining(graph, &groups_paths[1..], inserted_groups, n_solutions, n_solutions_tried);
//     }
// }
