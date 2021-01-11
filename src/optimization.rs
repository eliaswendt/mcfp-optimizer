use std::{collections::HashMap, sync::atomic::AtomicU64};

use petgraph::graph::{DiGraph, EdgeIndex};

use crate::model::{EdgeWeight, NodeWeight, path::Path};

/// try to split main path set into many tiny disjunct subsets
pub fn calculate_intersection_sets(tagged_paths: &Vec<(u64, Path)>) -> HashMap<Vec<EdgeIndex>, Vec<(u64, Path)>> {

    // hashmap with a vec of intersection edges as key and the tagged_paths as values sharing this intersection
    let mut intersection_sets: HashMap<Vec<EdgeIndex>, Vec<(u64, Path)>> = HashMap::new();

    // iterate over all tagged_paths
    for (cursor, (current_group_id, current_path)) in tagged_paths.iter().enumerate() {
        println!("comparing tagged_path {}/{} to all subsequent ones", cursor, tagged_paths.len());

        let current_paths_edges = current_path.edges_as_hash_set();

        // and comapre it to itself and all remaining tagged_paths in the vector (start is after current cursor position)
        for (other_group_id, other_path) in &tagged_paths[cursor+1..] {

            let intersecting_edges: Vec<EdgeIndex> = current_paths_edges
                .intersection(&other_path.edges_as_hash_set())
                .cloned()
                .collect();

            // if not empty add intersection to hashmap
            if !intersecting_edges.is_empty() {
                intersection_sets
                    .entry(intersecting_edges)
                    .or_insert(vec![(*current_group_id, current_path.clone())]) // intersection is new -> initialize with current
                    .push((*other_group_id, other_path.clone())); // then also push other
            }
        }
    }





    intersection_sets
}


pub fn recursive_path_straining(
    graph: &mut DiGraph<NodeWeight, EdgeWeight>, 
    groups_paths: &[(u64, Vec<Path>)], 
    inserted_groups: &mut Vec<u64>,
    n_solutions: &u128,
    n_solutions_tried: &mut u128,
) {
    if groups_paths.len() == 0 {
        // recursion anchor
        println!("inserted_groups.len()={}", inserted_groups.len());
        *n_solutions_tried += 1;
    } else {
        // println!("remaining_depth={}", groups_paths.len());

        let (group_id, group_paths) = groups_paths.first().unwrap();

        // push group.id onto the stack
        inserted_groups.push(*group_id);

        // make recursion call with each path known for this group
        for path in group_paths.iter() {
            if path.fits(graph) {
                path.strain(graph);

                recursive_path_straining(graph, &groups_paths[1..], inserted_groups, n_solutions, n_solutions_tried);

                path.relieve(graph);
            }
        }

        // remove group.id from the stack again
        inserted_groups.pop();

        // also try to leave-out this path by making a single recursive call without demanding
        recursive_path_straining(graph, &groups_paths[1..], inserted_groups, n_solutions, n_solutions_tried);
    }
}