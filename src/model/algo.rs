use petgraph::{EdgeDirection::{Outgoing}, graph::{NodeIndex, DiGraph}, visit::{IntoEdgeReferences, IntoEdges}};
use std::collections::{HashMap, HashSet};

use super::{EdgeWeight, NodeWeight, Object, group};
use group::Group;

pub fn max_flow(graph: &DiGraph<NodeWeight, EdgeWeight>, source_index: NodeIndex, sink_index: NodeIndex, groups: &HashMap<u64, Group>) {
    /*let paths: Vec<Vec<Object>> = Vec::new();
    let source_node = graph.node_weight(source_index).unwrap();

    for edge_ref in graph.edges_directed(source_index, Outgoing) { //(source_index, Outgoing).iter() {
        let path: Vec<EdgeIndex> = Vec::new();
        let capacity: u64 = 0;
        let edge = edge_ref.weight().;
        loop {
            //source_node
            let edge = graph.find_edge(a, b)
            let capacity = .unwrap().get.
        }
    }*/
}