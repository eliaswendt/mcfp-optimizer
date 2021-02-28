use petgraph::graph::DiGraph;

use crate::model::{group::Group, TimetableEdge, TimetableNode};

/// formalizing a system state
struct Manifestation {
    Vec<>
}

pub fn beam_search(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &Vec<Group>,
    beam_size: u64, // number of "parallel" hill-climb searches
) {

    // stores the index of the currently selected path in each group
    let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());

    
    
}
