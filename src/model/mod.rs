pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;
pub mod algo;

use group::Group;
use petgraph::{EdgeDirection::Outgoing, Graph, graph::{NodeIndex, EdgeIndex, DiGraph}, visit::{IntoEdgeReferences, IntoEdges}};
use petgraph::algo::{dijkstra, min_spanning_tree, all_simple_paths};
use petgraph::data::FromElements;
use petgraph::dot::{Dot, Config};
use std::{collections::{HashMap, HashSet}, iter::from_fn};

use std::fs::File;
use std::io::{prelude::*, BufWriter};

use crate::csv_reader;

/// Node Type of the DiGraph
#[derive(Debug, Clone)]
pub enum Node {
    Departure {
        trip_id: u64,
        time: u64,
        station_id: String
    },

    Arrival {
        trip_id: u64,
        time: u64
    },

    Station {
        station_id: String
    },

    Transfer {
        time: u64,
        station_id: String
    }
}

// impl std::fmt::Debug for Node {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Node")
//          .field("id", &self.id)
//          .finish()
//     }
// }

impl Node {
    // pub fn is_arrival(&self) -> bool {
    //     match self.kind {
    //         NodeType::Arrival => true,
    //         _ => false
    //     }
    // }

    // pub fn is_departure(&self) -> bool {
    //     match self.kind {
    //         NodeType::Departure => true,
    //         _ => false
    //     }
    // }

    // pub fn is_transfer(&self) -> bool {
    //     match self.kind {
    //         NodeType::Transfer => true,
    //         _ => false
    //     }
    // }

    pub fn get_time(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id, time, station_id} => Some(*time),
            Self::Arrival {trip_id, time} => Some(*time),
            Self::Transfer {time, station_id} => Some(*time),
            _ => None
        }
    }

    pub fn get_station(&self) -> Option<String> {
        match self {
            Self::Departure {trip_id, time, station_id} => Some(station_id.clone()),
            Self::Station {station_id} => Some(station_id.clone()),
            Self::Transfer {time, station_id} => Some(station_id.clone()),
            _ => None
        }
    }
}

/// Edge Type of the DiGraph
#[derive(Debug, Clone)]
pub enum Edge {
    RideToStation { // edge between departure and arrival
        duration: u64,
        capacity: u64,
        utilization: u64,
    },

    StayInTrain { // edge between arrival and departure in the same train (stay in the train)
        duration: u64
    },
    
    Embark, // edge between transfer node and departure

    Alight { // edge between arrival and transfer
        duration: u64
    },

    StayAtStation { // edge between two transfer nodes
        duration: u64
    },

    WalkToStation { // edge between arrival and next transfer node at other station
        duration: u64
    },

    StationRelation // edge between arrival/departure and station node
}


impl Edge {
    /// get duration of self, defaults to 0
    pub fn get_duration(&self) -> u64 {
        match self {
            Self::RideToStation{duration, capacity: _, utilization: _} => *duration,
            Self::StayInTrain{duration} => *duration,
            Self::Alight{duration} => *duration,
            Self::StayAtStation{duration} => *duration,
            Self::WalkToStation{duration} => *duration,
            _ => 0,
        }
    }

    /// get capacity of self, defaults to MAX
    pub fn get_capacity(&self) -> u64 {
        match self {
            Self::RideToStation{duration: _, capacity, utilization: _} => *capacity,
            _ => std::u64::MAX,
        }
    }

    /// increase utilization of this edge by <addend>
    pub fn increase_utilization(&mut self, addend: u64) {
        match self {
            Self::RideToStation{duration, capacity, utilization} => *utilization += addend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    pub fn set_utilization(&mut self, new_utilization: u64) {
        match self {
            Self::RideToStation{duration, capacity, utilization} => *utilization = new_utilization,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    /// get utilization of self, defaults to 0
    pub fn get_utilization(&self) -> u64 {
        match self {
            Self::RideToStation{duration: _, capacity: _, utilization} => *utilization,
            _ => 0 // other edges always return 0 utilization as they have unlimited capacity
        }
    }

    pub fn get_remaining_capacity(&self) -> u64 {
        match self {
            Self::RideToStation{duration: _, capacity, utilization} => *capacity - *utilization,
            _ => u64::MAX // other edges always return u64::MAX as they have unlimited capacity
        }
    }
}

pub enum Object {
    Edge(Edge),
    Node(Node)
}

pub enum ObjectIndex {
    NodeIndex(NodeIndex),
    EdgeIndex(EdgeIndex)
}

/// entire combined data model
pub struct Model {
    pub graph: DiGraph<Node, Edge>,

    // we need to store all departure nodes for all stations at all times
    stations_departures: HashMap<String, Vec<(u64, NodeIndex)>>,
    // only store one arrival main node for each station (each arrival points to this node)
    station_arrival_main_node_indices: HashMap<String, NodeIndex>,
}

impl Model {

    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let footpath_maps = csv_reader::read_to_maps(&format!("{}footpaths.csv", csv_folder_path));
        let station_maps = csv_reader::read_to_maps(&format!("{}stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}trips.csv", csv_folder_path));

        // convert each list of maps into a single map with multiple entries with id as key
        let footpaths_vec = footpath::Footpath::from_maps_to_vec(&footpath_maps);
        let mut stations_map = station::Station::from_maps_to_map(&station_maps);
        let trips_map = trip::Trip::from_maps_to_map(&trip_maps);

        let mut graph = DiGraph::new();
        let mut stations_departures = HashMap::with_capacity(stations_map.len());
        let mut station_arrival_main_node_indices = HashMap::with_capacity(stations_map.len());

        // iterate over all trips
        for (_, trip) in trips_map.iter() {

            // ARRIVAL NODE
            let arrival_node_index = graph.add_node(Node::Arrival {
                trip_id: trip.id,
                time: trip.arrival
            });

            // DEPARTURE NODE
            let departure_node_index = graph.add_node(Node::Departure {
                trip_id: trip.id,
                time: trip.departure,
                station_id: trip.from_station.clone()
            });

            // add these nodes to a station
            let to_station = stations_map.get_mut(&trip.to_station).unwrap();
            to_station.arrival_node_indices.insert(trip.id, arrival_node_index);

            let from_station = stations_map.get_mut(&trip.from_station).unwrap();
            from_station.departure_node_indices.insert(trip.id, departure_node_index);

            // connect stations of this trip
            graph.add_edge(departure_node_index, arrival_node_index, Edge::RideToStation {
                capacity: trip.capacity,
                duration: trip.arrival - trip.departure,
                utilization: 0
            });
        }

        // iterate over all stations
        for (station_id, station) in stations_map.iter_mut() {

            let station_arrival_main_node_index = graph.add_node(Node::Station {
                station_id: station_id.clone()
            });

            // iterate over all departures
            for (trip_id, departure_node_index) in station.departure_node_indices.iter() {

                let departure_node = graph.node_weight(*departure_node_index).unwrap();
                let departure_node_time = departure_node.get_time().unwrap();

                // DEPARTURE TRANSFER NODE (each departure also induces a corresponding departure node at the station)
                let departure_transfer_node_index = graph.add_node(Node::Transfer {
                    time: departure_node_time,
                    station_id: station_id.clone()
                });
                // edge between transfer of this station to departure
                graph.add_edge(departure_transfer_node_index, *departure_node_index, Edge::Embark);

                // add transfer node to list of transfer nodes of this station
                station.transfer_node_indices.push((departure_node_time, departure_transfer_node_index));

                // connect arrival of this trip to departure of this trip (if exists)
                // this edge represents staying in the same train
                match station.arrival_node_indices.get(trip_id) {
                    Some(arrival_node_index) => {
                        let arrival_node_time = graph.node_weight(*arrival_node_index).unwrap().get_time().unwrap();

                        graph.add_edge(*arrival_node_index, *departure_node_index, Edge::StayInTrain {
                            duration: departure_node_time - arrival_node_time
                        });
                    },
                    None => {}
                }
            }

            // sort transfer node list (by first tuple element -> time)
            station.transfer_node_indices.sort_unstable_by_key(|(key, _)| *key);

            // connect transfers with each other
            for transfer_node_indices in station.transfer_node_indices.windows(2) {
                graph.add_edge(transfer_node_indices[0].1, transfer_node_indices[1].1, Edge::StayAtStation {
                    duration: transfer_node_indices[1].0 - transfer_node_indices[0].0
                });
            }

            // iterate over all arrivals
            for (_, arrival_node_index) in station.arrival_node_indices.iter() {
                
                // connect arrival to station's main node
                graph.add_edge(*arrival_node_index, station_arrival_main_node_index, Edge::StationRelation);

                let arrival_node = graph.node_weight(*arrival_node_index).expect("Could not find node in graph");
                let arrival_node_time = arrival_node.get_time().unwrap();

                let earliest_transfer_time = arrival_node_time + station.transfer_time;

                // try to find next transfer node at this station
                for (transfer_timestamp, transfer_node_index) in station.transfer_node_indices.iter() {
                    if earliest_transfer_time <= *transfer_timestamp {
                        graph.add_edge(*arrival_node_index, *transfer_node_index, Edge::Alight {
                            duration: station.transfer_time
                        });
                        break // the loop
                    }
                }
            }

            // save refernces to all transfers and to arrival_main
            stations_departures.insert(station_id.clone(), station.transfer_node_indices.clone());
            station_arrival_main_node_indices.insert(station_id.clone(), station_arrival_main_node_index);
        }

        // iterate over all footpaths
        for footpath in footpaths_vec.iter() {
            let from_station = match stations_map.get(&footpath.from_station) {
                Some(from_station) => from_station,
                None => {
                    println!("footpath's from_station {} unknown", footpath.from_station);
                    continue // with next footpath
                }
            };

            let to_station = match stations_map.get(&footpath.to_station) {
                Some(to_station) => to_station,
                None => {
                    println!("footpath's to_station {} unknown", footpath.to_station);
                    continue // with next footpath
                }
            };

            // for every arrival at the from_station try to find the next transfer node at the to_station
            for arrival_node_index in from_station.arrival_node_indices.values() {
                let arrival_node_time = graph.node_weight(*arrival_node_index).unwrap().get_time().unwrap();

                // timestamp of arrival at the footpaths to_station
                let earliest_transfer_time = arrival_node_time + footpath.duration;

                let mut edge_added = false;

                // try to find next transfer node at to_station
                for (transfer_timestamp, transfer_node_index) in to_station.transfer_node_indices.iter() {
                    if earliest_transfer_time <= *transfer_timestamp {
                        graph.add_edge(*arrival_node_index, *transfer_node_index, Edge::WalkToStation {
                            duration: footpath.duration
                        });
                        edge_added = true;
                        break // the loop
                    }
                }

                if !edge_added {
                    println!("There couldn't be found any valid (time) transfer node for footpath from {} -> {}", footpath.from_station, footpath.to_station);
                }
            }
        }

        Self {
            graph,
            stations_departures,
            station_arrival_main_node_indices
        }
    }

    pub fn to_dot(&self) -> String {
        format!("{:?}", Dot::with_config(&self.graph, &[]))
    }


    pub fn find_solutions(&self, groups_csv_filepath: &str) {

        let group_maps = csv_reader::read_to_maps(groups_csv_filepath);
        let groups_map = group::Group::from_maps_to_map(&group_maps);
        let mut subgraph: DiGraph<Node, Edge> = Graph::new();
        // maps index of node in graph to index of node in subgraph
        let mut node_index_graph_subgraph_mapping: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        for (group_key, group_value) in groups_map.iter() {

            let from_node_index = self.find_start_node_index(&group_value.start, group_value.departure).expect("Could not find departure at from_station");
            let to_node_index = self.find_end_node_index(&group_value.destination).expect("Could not find arrival station");
    
            // max duration should depend on the original travel time
            let travel_time = (group_value.arrival - group_value.departure) as f64;
            let max_duration = (travel_time * 1.9) as u64; // factor to modify later if not a path could be found for all groups

            let paths = self.all_simple_paths(from_node_index, to_node_index, max_duration);
            
            let subgraph_paths = self.create_subgraph_from_paths(&mut subgraph, paths, &mut node_index_graph_subgraph_mapping);
    
            let dot_code = format!("{:?}", Dot::with_config(&subgraph, &[]));
    
            BufWriter::new(File::create(format!("graphs/subgraph_group_{}.dot", group_key)).unwrap()).write(
                dot_code.as_bytes()
            ).unwrap();
        }


        // todo: iterate groups, augment routes ... return solutions
    }


    pub fn find_start_node_index(&self, station_id: &str, start_time: u64) -> Option<NodeIndex> {
        match self.stations_departures.get(station_id) {
            Some(station_departures) => {
                
                // iterate until we find a departure time >= the time we want to start
                for (departure_time, departure_node_index) in station_departures.iter() {
                    if start_time <= *departure_time {
                        println!("{:?}", self.graph.node_weight(*departure_node_index).unwrap().get_station());
                        return Some(*departure_node_index);
                    }
                }

                // no departure >= start_time found
                None
            },

            // station not found
            None => None
        }
    }

    pub fn find_end_node_index(&self, station_id: &str) -> Option<NodeIndex> {
        match self.station_arrival_main_node_indices.get(station_id) {
            Some(station_arrival_main_node_index) => Some(*station_arrival_main_node_index),
            None => None
        }
    }


    /// creates a subgraph of self with only the part of the graph of specified paths
    pub fn create_subgraph_from_paths(&self, subgraph: &mut Graph<Node, Edge>, paths: Vec<Vec<NodeIndex>>, node_index_graph_subgraph_mapping: &mut HashMap<NodeIndex, NodeIndex>) -> Vec<Vec<ObjectIndex>> {
        //let mut subgraph = DiGraph::new();
        let mut subgraph_paths: Vec<Vec<ObjectIndex>> = Vec::new();

        // iterate all paths in graph
        for path in paths {

            let mut subgraph_path_indices: Vec<ObjectIndex> = Vec::new();
            let mut path_max_flow: u64 = std::u64::MAX;
            let mut path_edge_indices: Vec<EdgeIndex> = Vec::new();

            // iterate over all NodeIndex pairs in this path
            for graph_node_index_pair in path.windows(2) {

                // check if the first node already exists in subgraph
                let subgraph_node_a_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[0]) {
                    Some(subgraph_node_index) => *subgraph_node_index,
                    None => {
                        // clone NodeWeight from graph
                        let node_weight = self.graph.node_weight(graph_node_index_pair[0]).unwrap().clone();

                        // create new node in subgraph
                        let subgraph_node_index = subgraph.add_node(node_weight);
                        
                        // insert mapping into HashMap
                        node_index_graph_subgraph_mapping.insert(graph_node_index_pair[0], subgraph_node_index.clone());

                        subgraph_node_index
                    }
                };
                    
                // check if the second node already exists in subgraph
                let subgraph_node_b_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[1]) {
                    Some(subgraph_node_index) => *subgraph_node_index,
                    None => {
                        // clone NodeWeight from graph
                        let node_weight = self.graph.node_weight(graph_node_index_pair[1]).unwrap().clone();

                        // create new node in subgraph
                        let subgraph_node_index = subgraph.add_node(node_weight);
                        
                        // insert mapping into HashMap
                        node_index_graph_subgraph_mapping.insert(graph_node_index_pair[1], subgraph_node_index);

                        subgraph_node_index
                    }
                };
                
                // add outgoing node to path if path is empty
                if subgraph_path_indices.is_empty() {
                    subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_a_index));
                };

                // create edge if there was created at least one new node
                let subgraph_edge_weight = match subgraph.find_edge(subgraph_node_a_index, subgraph_node_b_index) {
                    Some(subgraph_edge_index) => {
                        // add edge to path
                        subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
                        path_edge_indices.push(subgraph_edge_index);
                        subgraph.edge_weight(subgraph_edge_index).unwrap()
                    },
                    None => {
                        let graph_edge_index = self.graph.find_edge(graph_node_index_pair[0], graph_node_index_pair[1]).unwrap();
                        let subgraph_edge_weight = self.graph.edge_weight(graph_edge_index).unwrap().clone();

                        let subgraph_edge_index = subgraph.add_edge(subgraph_node_a_index, subgraph_node_b_index, subgraph_edge_weight);
                        // add edge to path
                        subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
                        path_edge_indices.push(subgraph_edge_index);
                        subgraph.edge_weight(subgraph_edge_index).unwrap()
                    }
                };

                // update max_flow if edge capacity is smaller current path_max_flow
                let edge_remaining_flow = subgraph_edge_weight.get_capacity() - subgraph_edge_weight.get_utilization();
                if edge_remaining_flow < path_max_flow {
                    path_max_flow = edge_remaining_flow;
                };
                
                subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_b_index));
            };

            subgraph_paths.push(subgraph_path_indices);

            // set utilization to all edges of path
            for path_edge_index in path_edge_indices {
                subgraph.edge_weight_mut(path_edge_index).unwrap().increase_utilization(path_max_flow);
                //println!("{}, {}", path_max_flow, subgraph.edge_weight(path_edge_index).unwrap().get_utilization())
            }
        }

        subgraph_paths
    }

    fn all_simple_paths(&self, from_node_index: NodeIndex, to_node_index: NodeIndex, max_duration: u64) -> Vec<Vec<NodeIndex>> {

        // list of already visited nodes
        let mut visited = vec![from_node_index];

        // list of childs of currently exploring path nodes,
        // last elem is list of childs of last visited node
        let mut stack = vec![self.graph.neighbors_directed(from_node_index, Outgoing)];
        let mut durations: Vec<u64> = vec![0];
    
        let path_finder = from_fn(move || {
            while let Some(children) = stack.last_mut() {
                if let Some(child) = children.next() {
                    if durations.iter().sum::<u64>() < max_duration {
                        if child == to_node_index {
                            let path = visited
                                .iter()
                                .cloned()
                                .chain(Some(child))
                                .collect::<Vec<NodeIndex>>();
                            return Some(path);
                        } else if !visited.contains(&child) {
                            durations.push(self.graph.edge_weight(self.graph.find_edge(*visited.last().unwrap(), child).unwrap()).unwrap().get_duration());
                            visited.push(child);
                            stack.push(self.graph.neighbors_directed(child, Outgoing));
                        }
                    } else {
                        if child == to_node_index || children.any(|v| v == to_node_index) && durations.iter().sum::<u64>() >= max_duration { //&& visited.len() >= min_length {
                            let path = visited
                                .iter()
                                .cloned()
                                .chain(Some(child))
                                .collect::<Vec<NodeIndex>>();
                            return Some(path);
                        }
                        stack.pop();
                        visited.pop();
                        durations.pop();
                    }
                    
                } else {
                    stack.pop();
                    visited.pop();
                    durations.pop();
                }
            }
            
            None
        });
    
        path_finder.collect::<Vec<_>>()
    }
}