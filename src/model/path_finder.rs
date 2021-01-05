use std::iter::from_fn;

use indexmap::IndexSet;
use petgraph::{EdgeDirection::Outgoing, graph::{DiGraph, EdgeIndex, NodeIndex}};

use super::{EdgeWeight, NodeWeight};



/// iterative deeping depth-first-search (IDDFS)
fn all_paths_iddfs(
    graph: &DiGraph<NodeWeight, EdgeWeight>,
    from: NodeIndex,
    to: NodeIndex, // condition that determines whether goal node was found
    
    min_capacity: u64,
    max_duration: u64,

    n_steps: u64, 
    min_budget: u64,
    max_budget: u64 // maximum number of transfers to follow
) -> Vec<(u64, Vec<EdgeIndex>)> {

    // increase depth in 4 steps
    let depth_step = (max_budget - min_budget) / n_steps;

    for i in 0..n_steps+1 {

        let current_budget = min_budget + i * depth_step;

        println!("[iddfs()] trying with budget={}", current_budget);

        let result = all_paths_dfs_recursive(
            graph, 
            from, 
            to, 

            min_capacity, 
            max_duration, 
            current_budget
        );

        if result.len() > 0 {
            return result;
        }
    }

    println!("[iddfs()] giving up...");
    Vec::new()
}

// launcher of recursive implementation of dfs
// returns a vec of paths along with their remaining_duration
pub fn all_paths_dfs_recursive(
    graph: &DiGraph<NodeWeight, EdgeWeight>,
    from: NodeIndex,
    to: NodeIndex, // condition that determines whether goal node was found
    
    min_capacity: u64,
    max_duration: u64,
    max_budget: u64, // initial search budget (each edge has cost that needs to be payed)
) -> Vec<(u64, Vec<EdgeIndex>)> {

    // println!("all_paths_dfs(from={:?}, to={:?}, min_capacity={}, max_duration={})", from, to, min_capacity, max_duration);

    let mut paths = Vec::new();
    let mut visited = Vec::new();

    all_paths_dfs_recursive_helper(
        graph, 
        &mut paths,
        from, 
        to, 
        &mut visited, 

        min_capacity, 
        max_duration, 
        max_budget,
    );

    paths
}


fn all_paths_dfs_recursive_helper(
    graph: &DiGraph<NodeWeight, EdgeWeight>,
    paths: &mut Vec<(u64, Vec<EdgeIndex>)>, // paths found until now
    current: NodeIndex, 
    to: NodeIndex, 
    visited: &mut Vec<EdgeIndex>, // vec of visited edges (in order of visit)

    // recursion anchors (if zero)
    min_capacity: u64,
    remaining_duration: u64,
    remaining_budget: u64,
) {

    // println!("all_paths_dfs_recursive(current={:?}, goal={:?}, visited.len()={}, min_capacity={}, remaining_duration={})", current, to, visited.len(), min_capacity, remaining_duration);
    // println!("remaining_duration: {}", remaining_duration);

    if current == to {
        
        // take all edge indices (in order of visit) and insert them into a vec
        paths.push(
            (remaining_duration, visited.iter().cloned().collect())
        );

    } else {
        let mut walker = graph.neighbors_directed(current, Outgoing).detach();

        // iterate over all outgoing edges
        while let Some((next_edge, next_node)) = walker.next(graph) {

            let edge_weight = &graph[next_edge];
            let edge_weight_duration = edge_weight.get_duration();
            let edge_weight_cost = edge_weight.cost();

            if edge_weight_duration <= remaining_duration && edge_weight.get_remaining_capacity() >= min_capacity && edge_weight_cost <= remaining_budget {
                // edge can handle the minium required capacity and does not take longer then the remaining duration

                // add next_edge for next call
                visited.push(next_edge);

                &mut all_paths_dfs_recursive_helper(
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


fn all_simple_paths_dfs_dorian(graph: &'static DiGraph<NodeWeight, EdgeWeight>, from_node_index: NodeIndex, to_node_index: NodeIndex, max_duration: u64, max_rides: u64) -> impl Iterator<Item = (u64, Vec<EdgeIndex>)> {//Vec<(u64, Vec<EdgeIndex>)> {

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
                let mut duration = graph.edge_weight(child_edge_index).unwrap().get_duration();
                if durations.iter().sum::<u64>() + duration < max_duration && rides.iter().sum::<u64>() < max_rides {
                    if child_node_index == to_node_index {
                        let path = visited
                            .iter()
                            .cloned()
                            .chain(Some(child_edge_index))
                            .collect::<Vec<EdgeIndex>>();
                        return Some((max_duration - durations.iter().sum::<u64>() - duration, path));
                    } else if !visited.contains(&child_edge_index) {
                        let edge_weight = graph.edge_weight(child_edge_index).unwrap();
                        durations.push(edge_weight.get_duration());
                        // only count ride to station and walk to station as limit factor
                        // if edge_weight.is_ride_to_station() || edge_weight.is_walk_to_station() {
                        //     rides.push(1);
                        // } else {
                        //     rides.push(0);
                        // };
                        //rides.push(1);
                        visited.insert(child_edge_index);
                        stack.push(graph.neighbors_directed(child_node_index, Outgoing).detach());
                    }
                } else {
                    let mut children_any_to_node_index = false;
                    let mut edge_index = None;
                    let mut children_cloned = children.clone();
                    while let Some((c_edge_index, c_node_index)) = children_cloned.next(graph) {
                        if c_node_index == to_node_index {
                            children_any_to_node_index = true;
                            edge_index = Some(c_edge_index);
                            duration = graph.edge_weight(child_edge_index).unwrap().get_duration();
                            break;
                        }
                    }
                    if (child_node_index == to_node_index || children_any_to_node_index) 
                        && (durations.iter().sum::<u64>() + duration >= max_duration || rides.iter().sum::<u64>() >= max_rides) {
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



// launcher of recursive implementation of dfs
// currently not working (problems with last ancestor path element on stack (only first child works, siblings will have the same path))
// pub fn all_paths_dfs_iterative(
//     graph: &DiGraph<NodeWeight, EdgeWeight>,
//     from: NodeIndex,
//     to: NodeIndex, // condition that determines whether goal node was found
//     min_capacity: u64,
//     max_duration: u64,
//     max_depth: u64
// ) -> Vec<Vec<EdgeIndex>> {
//     // list of all found paths
//     let mut paths = Vec::new();

//     // saves path from root to current node
//     let mut ancestor_path: Vec<EdgeIndex> = Vec::with_capacity(max_depth as usize);


//     // (<nodes>, remaining_duration, depth-level)
//     let mut to_visit_children_groups: Vec<(Vec<NodeIndex>, u64, u64)> = vec![(vec![from], max_duration, 0)];


//     // iterate over groups that share the same ancestor path (and therefore also same remaining duration)
//     while let Some((children_nodes, remaining_duration, depth)) = to_visit_children_groups.pop() {

//         // current ancestor path always has the length of current depth
//         ancestor_path.truncate(depth as usize);

//         for child_node in children_nodes.iter() {
//             if *child_node == to {
//                 //
//             } else if depth < max_depth {

//                 let children_group = Vec::with_capacity(2);
//                 // iterate over all outgoing edges
//                 let mut walker = graph.neighbors_directed(child_node, Outgoing).detach();
//                 while let Some((next_edge, next_node)) = walker.next(graph) {
//                     children_group.
//                 }
//             }
//         }
        


//             } else if depth < max_depth {
                
//                 // println!("depth={} add edge={:?} to node={:?}", depth, next_edge, next_node);

//                 let edge_weight = &graph[next_edge];
//                 let edge_duration = edge_weight.get_duration();

//                 if edge_weight.get_remaining_capacity() >= min_capacity && edge_duration <= remaining_duration {
//                     // edge can handle the minium required capacity and does not take longer then the remaining duration        
//                     to_visit_children_groups.push((next_node, remaining_duration - edge_duration, depth+1));
//                     last_next_edge = Some(next_edge);
//                 }
//             }
//         }            

//     }

//     paths
// }