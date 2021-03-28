use std::{collections::HashMap, fs::File, time::Instant};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::io::BufReader;

pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;
pub mod path;
pub mod graph_weight;

use graph_weight::{TimetableNode, TimetableEdge};


use group::Group;

use petgraph::{
    dot::{Dot}, 
    graph::{
        NodeIndex,
        DiGraph
    }
};

use crate::csv_reader;

/// entire combined data model
#[derive(Serialize, Deserialize)]
pub struct Model {
    pub graph: DiGraph<TimetableNode, TimetableEdge>,

    // we need to store all transfer and arrival nodes for all stations at all times
    // required as entry-/endpoints for search
    stations_transfers: HashMap<u64, Vec<NodeIndex>>,

    // required for "in_trip" column of groups (groups could start in a train instead of a station)
    stations_arrivals: HashMap<u64, Vec<NodeIndex>>, 

    // connected to all arrivals of this station via zero-cost edge
    stations_main_arrival: HashMap<u64, NodeIndex>,
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
        let mut stations_main_arrival = HashMap::with_capacity(stations.len());

        // also save a HashMap of trips to parse group's "in_trip" column
        let trips = trip::Trip::from_maps_to_vec(&trip_maps);

        for trip in trips {
            trip.connect(&mut graph, &mut stations);
        }

        for (station_id, station) in stations.into_iter() {

            let station_name = station.name.clone();
            let (transfers, arrivals) = station.connect(&mut graph);

            // create main arrival node
            let main_arrival = graph.add_node(TimetableNode::MainArrival {
                station_id: station_id.clone(),
                station_name
            });

            // connect all arrival nodes to the main arrival
            for arrival in arrivals.iter() {
                // connect arrival to station's main node
                graph.add_edge(*arrival, main_arrival, TimetableEdge::MainArrivalRelation);
            }

            // save references to all transfers and to arrival_main
            stations_transfers.insert(station_id, transfers);
            stations_arrivals.insert(station_id, arrivals);
            stations_main_arrival.insert(station_id,main_arrival);
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
            stations_main_arrival,
        }
    }

    /// save model to file (for later runs)
    pub fn save_to_file(model: &Self, model_folder_path: &str) {
        print!("saving model to {} ... ", model_folder_path);
        let start = Instant::now();

        let writer = BufWriter::new(
            File::create(&format!("{}model.bincode", model_folder_path)).expect(&format!("Could not open file {}model.bincode", model_folder_path))
        );

        bincode::serialize_into(writer, model).expect("Could not dump model");
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

    pub fn find_paths_for_groups(&self, groups_csv_filepath: &str) -> Vec<Group> {

        // TODO: Falls die Gruppe an einer Station startet, muss in diesem Fall am Anfang die Stationsumstiegszeit berücksichtigt werden (kann man sich so vorstellen: die Gruppe steht irgendwo an der Station und muss erst zu dem richtigen Gleis laufen).
        // Befindet sich die Gruppe hingegen in einem Trip, hat sie zusätzlich die Möglichkeit, mit diesem weiterzufahren und erst später umzusteigen. (Würde man sie an der Station starten lassen, wäre die Stationsumstiegszeit nötig, um wieder in den Trip einzusteigen, in dem sie eigentlich schon ist - und meistens ist die Standzeit des Trips geringer als die Stationsumstiegszeit)
        // Habe auch die Formatbeschreibung im handcrafted-scenarios Repo entsprechend angepasst.

        let mut groups = Group::from_maps_to_vec(
            &csv_reader::read_to_maps(groups_csv_filepath));

        let groups_len = groups.len();
  
        
        let start = Instant::now();
        let mut n_groups_with_at_least_one_path: u64 = 0;

        for (index, group) in groups.iter_mut().enumerate() {

            print!("[group={}/{}]: ", index+1, groups_len);
            group.search_paths(&self);
            
            if group.paths.len() != 0 {
                n_groups_with_at_least_one_path += 1;
            }
        }

        println!(
            "Found at least one path for {}/{} groups ({}%) in {}s ({}min)", 
            n_groups_with_at_least_one_path, groups.len(),
            (100 * n_groups_with_at_least_one_path) / groups.len() as u64,
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

            // Number of MainArrival node found for Arrival node
            let mut num_main_arrival = 0;
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
                        let departure_before_arrival = node_a_weight.time().unwrap() <= node_b_weight.time().unwrap();
                        assert!(departure_before_arrival, format!("Node Departure has greater time as Arrival node! {} vs {}", node_a_weight.time().unwrap(), node_b_weight.time().unwrap()));
                        
                        // Departure node has only one outgoing edge
                        let one_outgoing = graph.neighbors(node_a_index).enumerate().count();
                        assert!(one_outgoing == 1, format!("Departure node has not one outgoing edge but {}", one_outgoing));
                    
                        // both nodes have same trip 
                        let same_trip = node_a_weight.trip_id().unwrap() == node_b_weight.trip_id().unwrap();
                        assert!(same_trip == true, format!("Departure node has not the same trip as Arrival node! {} vs {}", node_a_weight.trip_id().unwrap(), node_b_weight.trip_id().unwrap()));
                    },

                    TimetableNode::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => {

                        // Outgoing edge is WaitInTrain, Alight, Walk, or MainArrivalRelation
                        let edge_is_correct = edge_weight.is_wait_in_train() || edge_weight.is_alight()
                            || edge_weight.is_walk() || edge_weight.is_main_arrival_relation();
                        assert!(edge_is_correct, format!("Outgoing edge of arrival node is not WaitInStation, Alight, Walk, or MainArrivalRelation but {}!", edge_weight.kind_as_str()));
                        
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

                        // if edge is MainStationRelation -> node b is MainArrival
                        if edge_weight.is_main_arrival_relation() {
                            let arrival_to_main_arrival = node_b_weight.is_main_arrival();
                            assert!(arrival_to_main_arrival, format!("Node Arrival does not end in Transfer node after MainArrivalRelation edge but in {}!", node_b_weight.kind_as_str()));
                            num_main_arrival += 1;
                        }

                        // Arrival node has time before node b
                        if node_b_weight.is_departure() || node_b_weight.is_transfer() {
                            let arrival_before_departure_transfer = node_a_weight.time().unwrap() <= node_b_weight.time().unwrap();
                            assert!(arrival_before_departure_transfer, format!("Node Arrival has greater time as {} node! {} vs {}", node_b_weight.kind_as_str(), node_a_weight.time().unwrap(), node_b_weight.time().unwrap()));
                        }

                        // Arrival node and node b have same stations
                        if node_b_weight.is_departure() || node_b_weight.is_main_arrival() {
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

                            let same_time = node_a_weight.time().unwrap() == node_b_weight.time().unwrap();
                            assert!(same_time, format!("Transfer node and Departure node have not same time! {} vs. {}", node_a_weight.time().unwrap(), node_b_weight.time().unwrap()));
                        
                            num_board += 1;
                        }

                        // if edge is WaitAtStation -> node b is Transfer node and node b has time greater or equal node a
                        if edge_weight.is_wait_at_station() {
                            let transfer_to_transfer = node_b_weight.is_transfer();
                            assert!(transfer_to_transfer, format!("Node Transfer does not end in Transfer node after WaitAtStation edge but in {}!", node_b_weight.kind_as_str()));
                        
                            let transfer_before_transfer = node_a_weight.time().unwrap() <= node_b_weight.time().unwrap();
                            assert!(transfer_before_transfer, format!("Transfer node has not time less or equal Transfer node! {} vs. {}", node_a_weight.time().unwrap(), node_b_weight.time().unwrap()));
                        }

                        // both nodes have same station
                        let same_stations = node_a_weight.station_id() == node_b_weight.station_id();
                        assert!(same_stations, format!("Transfer node and {} node have not same station! {} vs. {}", node_b_weight.kind_as_str(), node_a_weight.station_id(), node_b_weight.station_id()));                   
                    },
                    TimetableNode::MainArrival {station_id: _, station_name: _} => {
                        
                        // todo: Panic because MainArrival node has no outgoing edges
                        assert!(false, "MainArrival node has outgoing edge!")
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
                    
                    // Only one MainArrival node found
                    if num_main_arrival != 1 {
                        println!("Outgoing edges:");
                        let mut children = graph.neighbors_directed(node_a_index, Outgoing).detach();
                        while let Some((child_edge_index, child_node_index)) = children.next(&graph) {
                            println!("{:?}", graph.edge_weight(child_edge_index).unwrap());
                        }
                    }
                    assert!(num_main_arrival == 1, format!("Arrival node has {} MainArrival nodes instead of 1!", num_main_arrival));

                    // Max one WaitInTrain outgoing edge per Arrival
                    assert!(num_wait_in_train <= 1, format!("Arrival node has {} outgoing WaitInTrain edges instead of 0 or 1!", num_wait_in_train));
                },
                TimetableNode::Transfer {time: _, station_id: _, station_name: _} => {

                    // Only one outoging board edge
                    assert!(num_board == 1, format!("Transfer node has {} outgoing Board edges instead of 1!", num_board));
                },
                TimetableNode::MainArrival {station_id: _, station_name: _} => {
                        
                    // has no outgoing edges
                    let outoging_edge_count = graph.edges_directed(node_a_index, Outgoing).count();
                    assert!(outoging_edge_count == 0, format!("MainArrival node has {} outgoing Board edges instead of 0!", outoging_edge_count));
                }
            }
        }

        println!("[validate_graph_integrity()]: passed ({}ms)", start.elapsed().as_millis());
    }

    // #[test]
    // fn validate_groups_paths_integrity() {

    //     let model = Model::with_stations_trips_and_footpaths("real_data");
    //     let graph = &model.graph;
    //     let groups = &model.find_paths_for_groups("real_data/groups.csv");

    //     for group in groups {
    //         let paths = &group.paths;

    //         let from = model
    //             .find_start_node_index(&group.start, group.departure)
    //             .expect("Could not find departure at from_station");
    //         let to = model
    //             .find_end_node_index(&group.destination)
    //             .expect("Could not find destination station");

    //         for path in paths {
    //             let edges = &path.edges;

    //             let mut current_node_index = from;

    //             'outer: for edge in edges {
    //                 let mut walker = graph.neighbors_directed(current_node_index, Outgoing).detach();
    //                 while let Some((edge_index, node_index)) = walker.next(graph) {
    //                     if *edge == edge_index {
    //                         current_node_index = node_index;
    //                         continue 'outer;
    //                     }
    //                 }
    //                 assert!(false, "Path is not correctly connected!")
    //             }

    //             assert!(current_node_index == to, "Last node is not correct!")
    //         }
    //     }
    // }
}



// unused code from previous version of find_solutions()


        // // create a HashSet of all EdgeIndices from all group's paths
        // let mut all_edges: HashSet<EdgeIndex> = HashSet::new();
        // for group_paths in groups_paths.values() {
        //     for (_, path) in group_paths.iter() {
        //         for edge_index in path.iter() {
        //             all_edges.insert(*edge_index);
        //         }
        //     }
        // }

        // let subgraph = path::create_subgraph_from_edges(&self.graph, &all_edges);





            // sort by remaining time and number of edges
            // sort paths by remaining duration (highest first)
            // paths_recursive.sort_by(|(x_remaining_duration, x_path), (y_remaining_duration, y_path)| {
            //     match x_remaining_duration.cmp(y_remaining_duration).reverse() {
            //         std::cmp::Ordering::Equal => x_path.len().cmp(&y_path.len()), // prefer less edges -> should be less transfers
            //         other => other, 
            //     }
            // });

            
            //let paths_recursive = self.all_simple_paths_dfs_dorian(from_node_index, to_node_index, max_duration, 5);

            // let mut only_paths = Vec::new();
            // for (_, path) in paths_recursive {
            //     only_paths.push(path.clone())
            // }

            

            // let all_edges_in_paths_recursive: HashSet<EdgeIndex> = only_paths.iter().flatten().cloned().collect();
            // if all_edges_in_paths_recursive.len() > 0 {
            //     let subgraph = self.build_subgraph_with_edges(&all_edges_in_paths_recursive);
            //     println!("node_count_subgraph={}, edge_count_subgraph={}", subgraph.node_count(), subgraph.edge_count());

            //     BufWriter::new(File::create(format!("graphs/groups/{}.dot", group_value.id)).unwrap()).write(
            //         format!("{:?}", Dot::with_config(&subgraph, &[])).as_bytes()
            //     ).unwrap();
            // }

            // let subgraph_paths = self.create_subgraph_with_nodes(&mut subgraph, paths_recursive, &mut node_index_graph_subgraph_mapping);
    
            // let dot_code = format!("{:?}", Dot::with_config(&subgraph, &[]));
    
            // BufWriter::new(File::create(format!("graphs/subgraph_group_{}.dot", group_key)).unwrap()).write(
            //     dot_code.as_bytes()
            // ).unwrap();
        // }

        // let dot_code = format!("{:?}", Dot::with_config(&subgraph, &[]));
    
        // BufWriter::new(File::create(format!("graphs/subgraph_complete.dot")).unwrap()).write(
        //     dot_code.as_bytes()
        // ).unwrap();


        // todo: iterate groups, augment routes ... return solutions