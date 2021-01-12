// use std::{collections::HashMap, sync::atomic::AtomicU64};

// use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

// use crate::model::{TimetableEdge, TimetableNode, group::Group, path::Path};





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