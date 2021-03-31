use std::{collections::{HashMap, HashSet}, fs::File, sync::{Arc, Mutex}, time::Instant};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::io::BufReader;
use crossbeam_utils::thread;

pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;
pub mod path;
pub mod graph_weight;

use graph_weight::{TimetableNode, TimetableEdge};


use group::Group;

use petgraph::{dot::{Dot}, graph::{DiGraph, EdgeIndex, NodeIndex}};

use crate::csv_reader;

/// entire combined data model
#[derive(Serialize, Deserialize)]
pub struct Model {
    pub graph: DiGraph<TimetableNode, TimetableEdge>,

    // we need to store all transfer and arrival nodes for all stations at all times
    // required as entry-/endpoints for path search
    pub stations_transfers: HashMap<u64, Vec<NodeIndex>>,

    // required for "in_trip" column of groups (groups could start in a train instead of a station)
    pub stations_arrivals: HashMap<u64, Vec<NodeIndex>>
}

impl Model {

    /// Build a timetable model (graph) from a folder that contains the following files:
    ///
    /// `stations.csv`, `footpaths.csv`, `trips.csv`
    pub fn with_stations_trips_and_footpaths(csv_folder_path: &str) -> Self {

        let start = Instant::now();

        // read all input data CSVs
        let station_maps = csv_reader::read_to_maps(&format!("{}/stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}/trips.csv", csv_folder_path));
        let footpath_maps = csv_reader::read_to_maps(&format!("{}/footpaths.csv", csv_folder_path));

        // initialize graph
        let mut graph = DiGraph::new();

        let mut stations = station::Station::from_maps_to_map(&station_maps);
        let mut stations_transfers = HashMap::with_capacity(stations.len());
        let mut stations_arrivals = HashMap::with_capacity(stations.len());

        // also save a HashMap of trips to parse group's "in_trip" column
        let trips = trip::Trip::from_maps_to_vec(&trip_maps);

        for trip in trips {
            trip.connect(&mut graph, &mut stations);
        }

        for (station_id, station) in stations.into_iter() {

            let station_name = station.name.clone();
            let (transfers, arrivals) = station.connect(&mut graph);

            // save references to all transfers and to arrival_main
            stations_transfers.insert(station_id, transfers);
            stations_arrivals.insert(station_id, arrivals);
        }

        let mut successful_footpath_counter = 0;
        let mut failed_footpath_counter = 0;

        // iterate over all footpaths
        for footpath in footpath::Footpath::from_maps_to_vec(&footpath_maps) {

            let from_station_arrivals = stations_arrivals.get(&footpath.from_station).unwrap();
            let to_station_transfers = stations_transfers.get(&footpath.to_station).unwrap();

            // connect stations via footpaths
            let (
                successful_footpaths,
                failed_footpaths
            ) = footpath.connect(&mut graph, from_station_arrivals, to_station_transfers);

            successful_footpath_counter += successful_footpaths;
            failed_footpath_counter += failed_footpaths;
        }
        println!("successful_footpaths: {}, failed_footpaths: {}", successful_footpath_counter, failed_footpath_counter);


        println!(
            "[with_stations_trips_and_footpaths()]: done ({}ms), graph.node_count()={}, graph.edge_count()={}", 
            start.elapsed().as_millis(),
            graph.node_count(), 
            graph.edge_count()
        );

        Self {
            graph,
            stations_transfers,
            stations_arrivals,
        }
    }

    /// save model to file (for later runs)
    pub fn save_to_file(&self, model_folder_path: &str) {
        print!("saving model to {} ... ", model_folder_path);
        let start = Instant::now();

        let writer = BufWriter::new(
            File::create(&format!("{}model.bincode", model_folder_path)).expect(&format!("Could not open file {}model.bincode", model_folder_path))
        );

        bincode::serialize_into(writer, self).expect("Could not dump model");
        //serde_json::to_writer(writer, model).expect("Could not dump model");

        println!("done ({}ms)", start.elapsed().as_millis());
    }

    /// load model from file (from previous run)
    pub fn load_from_file(model_folder_path: &str) -> Self {
        print!("loading model from {} ... ", model_folder_path);
        let start = Instant::now();

        let reader = BufReader::new(
            File::open(&format!("{}model.bincode", model_folder_path)).expect(&format!("Could not open file {}model.bincode", model_folder_path))
        );
        let model: Self = bincode::deserialize_from(reader).expect("Could not load model from file!");
        // let model: Self = serde_json::from_reader(reader).expect("Could not load model from file!");


        println!("done ({}ms)", start.elapsed().as_millis());

        model
    }

    /// create graviz dot code of model's graph 
    pub fn save_dot_code_to(model: &Self, filepath: &str) {
        let dot_code = format!("{:?}", Dot::with_config(&model.graph, &[]));

        BufWriter::new(
            File::create(filepath).expect(&format!("Could not create dot-file at {}", filepath))
        ).write(dot_code.as_bytes()).unwrap();
    }

    /// builds subgraph that only contains nodes connected by edges
    pub fn create_subgraph_from_edges(
        &self,
        edges: HashSet<EdgeIndex>,
        filepath: &str,
    ) {
        let new_graph = self.graph.filter_map(
            |node_index, node_weight| {
                // check if at least one incoming/outgoing edge of current node is in HashSet of edges
                let mut walker = self.graph.neighbors_undirected(node_index).detach();
                while let Some((current_edge, _)) = walker.next(&self.graph) {
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
        );

        let dot_code = format!("{:?}", Dot::with_config(&new_graph, &[]));

        BufWriter::new(
            File::create(filepath).expect(&format!("Could not create dot-file at {}", filepath)),
        )
        .write(dot_code.as_bytes())
        .unwrap();
    }

    pub fn find_paths_for_groups(&self, groups_csv_filepath: &str, search_budget: &[u64], n_threads: usize) -> Vec<Group> {

        // TODO: Falls die Gruppe an einer Station startet, muss in diesem Fall am Anfang die Stationsumstiegszeit berücksichtigt werden (kann man sich so vorstellen: die Gruppe steht irgendwo an der Station und muss erst zu dem richtigen Gleis laufen).
        // Befindet sich die Gruppe hingegen in einem Trip, hat sie zusätzlich die Möglichkeit, mit diesem weiterzufahren und erst später umzusteigen. (Würde man sie an der Station starten lassen, wäre die Stationsumstiegszeit nötig, um wieder in den Trip einzusteigen, in dem sie eigentlich schon ist - und meistens ist die Standzeit des Trips geringer als die Stationsumstiegszeit)
        // Habe auch die Formatbeschreibung im handcrafted-scenarios Repo entsprechend angepasst.

        let unprocessed_groups = Arc::new(
            Mutex::new(
                Group::from_maps_to_vec(&csv_reader::read_to_maps(groups_csv_filepath))
            )
        );
            
        let processed_groups = Arc::new(Mutex::new(Vec::with_capacity(unprocessed_groups.lock().unwrap().len())));
  
        let start = Instant::now();

        thread::scope(|s| {
            // use multiple threads to find paths
            for _ in 0..n_threads {

                let unprocessed_groups = Arc::clone(&unprocessed_groups);
                let processed_groups = Arc::clone(&processed_groups);

                s.spawn(move |_| {
                    loop {
                        let group_option = unprocessed_groups.lock().unwrap().pop();

                        match group_option {
                            Some(mut group) => {
                                print!("[group={}]: ", group.id);
                                group.search_paths(&self, search_budget);

                                // add processed group to processed vec
                                processed_groups.lock().unwrap().push(group)

                            },
                            None => {
                                // no group left in unprocessed vec
                                break
                            }
                        }
                    }
                });
            }
        }).unwrap();

        let groups = processed_groups.lock().unwrap().clone();

        let n_groups_with_at_least_one_path = groups.iter().filter(|g| !g.paths.is_empty()).count();

        println!(
            "Found at least one path for {}/{} groups ({}%) in {}s ({}min)", 
            n_groups_with_at_least_one_path, groups.len(),
            (100 * n_groups_with_at_least_one_path) / groups.len(),
            start.elapsed().as_secs(),
            start.elapsed().as_secs() / 60
        );

        groups
    }
}

#[cfg(test)]
mod tests {
    use petgraph::EdgeDirection::Outgoing;

    use super::*;

    /// Panics if invalid
    #[test]
    fn validate_graph_integrity() {

        let model = Model::with_stations_trips_and_footpaths("real_data");
        let graph = model.graph;

        let start = Instant::now();

        for node_a_index in graph.node_indices() {
            let node_a_weight = graph.node_weight(node_a_index).unwrap();
            
            let mut children = graph.neighbors_directed(node_a_index, Outgoing).detach();

            // Number of WaitInTrain edges for Arrival node
            let mut num_wait_in_train = 0;
            // Number of Board edges for Transfer node
            let mut num_board = 0;

            while let Some((edge_index, child_b_index)) = children.next(&graph){
                // Check valid successor
                let edge_weight = graph.edge_weight(edge_index).unwrap();
                let node_b_weight = graph.node_weight(child_b_index).unwrap();


                // check node relation
                match node_a_weight {
                    TimetableNode::Departure {trip_id: _, time: _, station_id: _, station_name: _} => {

                        // Departure outgoing edge is ride
                        let edge_is_ride = edge_weight.is_trip();
                        assert!(edge_is_ride, format!("Outgoing edge of departure node is not Ride but {}!", edge_weight.kind_as_str()));
                        
                        // Outgoing Edge ends in Arrival node
                        let departure_to_arrival =  node_b_weight.is_arrival();
                        assert!(departure_to_arrival, format!("Node Departure does not end in Arrival node but in {}!", node_b_weight.kind_as_str()));
                        
                        // Departure time is before Arrival time
                        let departure_before_arrival = node_a_weight.time() <= node_b_weight.time();
                        assert!(departure_before_arrival, format!("Node Departure has greater time as Arrival node! {} vs {}", node_a_weight.time(), node_b_weight.time()));
                        
                        // Departure node has only one outgoing edge
                        let one_outgoing = graph.neighbors(node_a_index).enumerate().count();
                        assert!(one_outgoing == 1, format!("Departure node has not one outgoing edge but {}", one_outgoing));
                    
                        // both nodes have same trip 
                        let same_trip = node_a_weight.trip_id().unwrap() == node_b_weight.trip_id().unwrap();
                        assert!(same_trip == true, format!("Departure node has not the same trip as Arrival node! {} vs {}", node_a_weight.trip_id().unwrap(), node_b_weight.trip_id().unwrap()));
                    },

                    TimetableNode::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => {

                        // Outgoing edge is WaitInTrain, Alight, or Walk
                        let edge_is_correct = edge_weight.is_wait_in_train() || edge_weight.is_alight()
                            || edge_weight.is_walk();
                        assert!(edge_is_correct, format!("Outgoing edge of arrival node is not WaitInStation, Alight or Walk but {}!", edge_weight.kind_as_str()));
                        
                        // if edge is WaitInTrain -> Nodes have same trip and node b is departure
                        if edge_weight.is_wait_in_train() {
                            let arrival_to_departure = node_b_weight.is_departure();
                            assert!(arrival_to_departure, format!("Node Arrival does not end in Departure node after WaitInTrain edge but in {}!", node_b_weight.kind_as_str()));
                            
                            num_wait_in_train += 1;

                            // same trip id
                            let same_trip = node_a_weight.trip_id().unwrap() == node_b_weight.trip_id().unwrap();
                            assert!(same_trip == true, format!("Arrival node has not the same trip as Departure node for WaitInStation edge! {} vs {}", node_a_weight.trip_id().unwrap(), node_b_weight.trip_id().unwrap()));
                        }

                        // if edge is Alight -> node b is transfer
                        if edge_weight.is_alight() {
                            let arrival_to_transfer = node_b_weight.is_transfer();
                            assert!(arrival_to_transfer, format!("Node Arrival does not end in Transfer node after Alight edge but in {}!", node_b_weight.kind_as_str()));
                        }

                        // if edge is Walk -> node b is transfer
                        if edge_weight.is_walk() {
                            let arrival_to_walk = node_b_weight.is_transfer();
                            assert!(arrival_to_walk, format!("Node Arrival does not end in Transfer node after Walk edge but in {}!", node_b_weight.kind_as_str()));
                        }

                        // Arrival node has time before node b
                        if node_b_weight.is_departure() || node_b_weight.is_transfer() {
                            let arrival_before_departure_transfer = node_a_weight.time() <= node_b_weight.time();
                            assert!(arrival_before_departure_transfer, format!("Node Arrival has greater time as {} node! {} vs {}", node_b_weight.kind_as_str(), node_a_weight.time(), node_b_weight.time()));
                        }

                        // Arrival node and node b have same stations
                        if node_b_weight.is_departure() {
                            // same stations
                            let same_stations = node_a_weight.station_id() == node_b_weight.station_id();
                            assert!(same_stations, format!("Arrival node and {} node have not same station! {} vs. {}", node_b_weight.kind_as_str(), node_a_weight.station_id(), node_b_weight.station_id()));
                        }
                    },
                    TimetableNode::Transfer {time: _, station_id: _, station_name: _} => {

                        // Outgoing edge is Board or WaitAtStation
                        let edge_is_correct = edge_weight.is_board() || edge_weight.is_wait_at_station();
                        assert!(edge_is_correct, format!("Outgoing edge of Transfer node is not Board, or WaitAtStation but {}!", edge_weight.kind_as_str()));
                        
                        // if edge is Board -> node b is Departure node and both have same time
                        if edge_weight.is_board() {
                            let transfer_to_departure = node_b_weight.is_departure();
                            assert!(transfer_to_departure, format!("Node Transfer does not end in Departure node after Board edge but in {}!", node_b_weight.kind_as_str()));

                            let same_time = node_a_weight.time() == node_b_weight.time();
                            assert!(same_time, format!("Transfer node and Departure node have not same time! {} vs. {}", node_a_weight.time(), node_b_weight.time()));
                        
                            num_board += 1;
                        }

                        // if edge is WaitAtStation -> node b is Transfer node and node b has time greater or equal node a
                        if edge_weight.is_wait_at_station() {
                            let transfer_to_transfer = node_b_weight.is_transfer();
                            assert!(transfer_to_transfer, format!("Node Transfer does not end in Transfer node after WaitAtStation edge but in {}!", node_b_weight.kind_as_str()));
                        
                            let transfer_before_transfer = node_a_weight.time() <= node_b_weight.time();
                            assert!(transfer_before_transfer, format!("Transfer node has not time less or equal Transfer node! {} vs. {}", node_a_weight.time(), node_b_weight.time()));
                        }

                        // both nodes have same station
                        let same_stations = node_a_weight.station_id() == node_b_weight.station_id();
                        assert!(same_stations, format!("Transfer node and {} node have not same station! {} vs. {}", node_b_weight.kind_as_str(), node_a_weight.station_id(), node_b_weight.station_id()));                   
                    }
                }
            }

            // check node on its own
            match node_a_weight {
                TimetableNode::Departure {trip_id: _, time: _, station_id: _, station_name: _} => {
                    
                    // Exactly one outgoing edge
                    let num_edges = graph.edges_directed(node_a_index, Outgoing).count();
                    assert!(num_edges == 1, format!("Departure node has {} outgoing edges instead of one!", num_edges));
                },
                TimetableNode::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => {
                    
                    // Max one WaitInTrain outgoing edge per Arrival
                    assert!(num_wait_in_train <= 1, format!("Arrival node has {} outgoing WaitInTrain edges instead of 0 or 1!", num_wait_in_train));
                },
                TimetableNode::Transfer {time: _, station_id: _, station_name: _} => {

                    // Only one outoging board edge
                    assert!(num_board == 1, format!("Transfer node has {} outgoing Board edges instead of 1!", num_board));
                }
            }
        }

        println!("[validate_graph_integrity()]: passed ({}ms)", start.elapsed().as_millis());
    }

    #[test]
    fn validate_groups_paths_integrity() {

        let snapshot_folder_path = "snapshot/";
        let model = Model::load_from_file(snapshot_folder_path);
        let graph = &model.graph;
        let groups = Group::load_from_file(snapshot_folder_path);

        // test all groups
        for group in groups {

            // get all paths of group
            let paths = &group.paths;

            // get start and destination station id
            let start_station_id = group.start_station_id;
            let destination_station_id = group.destination_station_id;

            // find start node id
            let start: NodeIndex = match group.in_trip {
                Some(in_trip) => {
                    // in_trip is set -> start at arrival of current trip
    
                    // println!("start={}, in_trip={}, departure={}", self.start, in_trip, self.departure);
    
                    // FIRST: get all arrival nodes of the start station
                    let start_station_arrivals =
                        model.stations_arrivals.get(&group.start_station_id).unwrap();
    
                    // SECOND: search all arrivals for trip_id == in_trip AND time == start at start station
                    let mut selected_station_arrival = None;
                    for start_station_arrival in start_station_arrivals.iter() {
                        let arrival = &model.graph[*start_station_arrival];
    
                        if arrival.trip_id().unwrap() == in_trip
                            && arrival.time() == group.departure_time
                        {
                            selected_station_arrival = Some(*start_station_arrival);
                            // println!("Found arrival={:?}", arrival);
                            break;
                        }
                    }
    
                    selected_station_arrival.expect(&format!(
                        "Could not find arrival for in_trip={} and departure={}",
                        in_trip, group.departure_time
                    ))
                }
                None => {
                    // in_trip is not set -> start at station transfer
    
                    let mut selected_station_transfer = None;
    
                    match model.stations_transfers.get(&group.start_station_id) {
                        Some(station_transfers) => {
                            // iterate until we find a departure time >= the time we want to start
                            for station_transfer in station_transfers.iter() {
                                if group.departure_time <= model.graph[*station_transfer].time()
                                {
                                    selected_station_transfer = Some(*station_transfer);
                                    break;
                                }
                            }
                        }
                        None => {}
                    }
    
                    selected_station_transfer.expect("Could not find departure at from_station")
                }
            };
    
            // find destination station name
            let destination_station_name = model.graph
                [model.stations_arrivals.get(&group.destination_station_id).unwrap()[0]]
                .station_name();

            let start_timetable_node = &graph[start];
            // test if start node's station id equals groups' start_station_id
            assert!(start_timetable_node.station_id() == start_station_id, "Start node has not correct station id!");
            // test if start node is transfer or arrival
            assert!(start_timetable_node.is_arrival() || start_timetable_node.is_transfer(), "Start station is neither arrival nor transfer node!");
            // test if time of start node is >= groups departure time
            assert!(start_timetable_node.time() >= group.departure_time, "Start node's time is smaller than group's departure time!");

            for path in paths {

                let edges = &path.edges;

                // test if first edge is correct node
                assert!(graph.edge_endpoints(edges[0]).unwrap().0 == start, "First node in path does not equal start node!");

                let mut current_node_index = start;

                'outer: for edge in edges {
                    let mut walker = graph.neighbors_directed(current_node_index, Outgoing).detach();
                    while let Some((edge_index, node_index)) = walker.next(graph) {
                        if *edge == edge_index {
                            current_node_index = node_index;
                            continue 'outer;
                        }
                    }
                    assert!(false, "Path is not correctly connected!")
                }
                assert!(current_node_index == graph.edge_endpoints(*edges.last().unwrap()).unwrap().1, "Last edge node in path is not current edge!");
                assert!(model.graph[current_node_index].station_id() == destination_station_id, "Last station id is not correct!");
                assert!(model.graph[current_node_index].station_name() == destination_station_name, "Last station name is not correct!");
                assert!(model.graph[current_node_index].is_arrival() || model.graph[current_node_index].is_transfer(), "Last node is not arrival!")
            }
        }
    }
}
