fn depth_limited_search(graph: &DiGraph<Node, Edge>, start_node_index: NodeIndex, destination_station: &str, max_costs: &u64) -> DiGraph<Node, Edge> {
    let current_path = vec![start_node_index];
    let paths = all_simple_paths(graph, start_node_index, destination_station, limit_costs.clone());
    println!("All found paths for max costs {}: {:?}", (max_costs, paths));
    let subgraph = create_graph(graph, paths);
    return subgraph
}

fn find_solution(graph: &DiGraph<Node, Edge>, group: &Group) -> DiGraph<Node, Edge> {
    let mut start_node_index: Option<NodeIndex> = None;
    
    // find node index with same station as group and minimal time greater than group departure time
    for node_index in graph.node_indices().into_iter() {
        let current_node = graph.node_weight(node_index).unwrap();
        let pred = current_node.get_station() == group.get_start() &&
            current_node.get_time() >= group.get_departure_time() && current_node.is_transfer();
        if pred {
            start_node_index = match start_node_index {
                Some(index) => {
                    let start_node = graph.node_weight(index).unwrap();
                    if current_node.get_time() < start_node.get_time() {
                        Some(node_index)
                    } else {
                        Some(index)
                    }
                },
                None => Some(node_index),
            }
        }
    }

    match start_node_index {
        Some(index) => println!("{:?}", graph.node_weight(index).unwrap()),
        None => {
            println!("Start node not found!");
            exit(-1);
        },   
    }

    let destination_station = group.get_destination();
    //path search
    return depth_limited_search(graph,&start_node_index.unwrap(), destination_station, &80) // limit_costs TODO 
}



fn goal_test(graph: &DiGraph<Node, Edge>, node_index: &NodeIndex, destination_station: &str) -> bool {
    graph.node_weight(*node_index).unwrap().get_station() == destination_station
}