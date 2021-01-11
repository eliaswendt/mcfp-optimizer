use petgraph::graph::DiGraph;

use crate::model::{EdgeWeight, NodeWeight, path::Path};


pub fn recursive_path_straining(
    graph: &mut DiGraph<NodeWeight, EdgeWeight>, 
    groups_paths: &[(u64, Vec<Path>)], 
    inserted_groups: &mut Vec<u64>
) {
    if groups_paths.len() == 0 {
        // recursion anchor
        println!("inserted_groups.len()={}", inserted_groups.len());
    } else {
        println!("remaining_depth={}", groups_paths.len());

        let (group_id, group_paths) = groups_paths.first().unwrap();

        for path in group_paths.iter() {
            if path.fits(graph) {
                path.strain(graph);

                recursive_path_straining(graph, &groups_paths[1..], inserted_groups);

                path.relieve(graph);
            }
        } 
    }
}