pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;

use group::Group;
use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::algo::{dijkstra, min_spanning_tree};
use petgraph::data::FromElements;
use petgraph::dot::{Dot, Config};
use std::collections::{HashMap};

use std::fs::File;
use std::io::{prelude::*, BufWriter};

use crate::csv_reader;

#[derive(Debug)]
enum NodeType {
    Departure,
    Arrival,
    Transfer
}

#[derive(Debug)]
pub struct Node {
    station: String,
    trip_id: u64,
    time: u64, // time of arrival/departure

    kind: NodeType // type of this node (departure, arrival or stay)
}

// impl std::fmt::Debug for Node {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Node")
//          .field("id", &self.id)
//          .finish()
//     }
// }

impl Node {
    pub fn is_arrival(&self) -> bool {
        match self.kind {
            NodeType::Arrival => true,
            _ => false
        }
    }

    pub fn is_departure(&self) -> bool {
        match self.kind {
            NodeType::Departure => true,
            _ => false
        }
    }

    pub fn is_transfer(&self) -> bool {
        match self.kind {
            NodeType::Transfer => true,
            _ => false
        }
    }
}

/// Edge Type of the DiGraph
#[derive(Debug)]
pub enum Edge {
    Trip { // edge between departure and arrival
        duration: u64,
        capacity: u64
    },

    Stay { // edge between arrival and departure in the same train (stay in the train)
        duration: u64
    },
    
    Embark {}, // edge between transfer node and departure

    Alight { // edge between arrival and transfer
        duration: u64
    },

    TransferWait { // edge between two transfer nodes
        duration: u64
    },

    TransferWalk { // edge between arrival and next transfer node at other station
        duration: u64
    }
}




/// entire combined data model
pub struct Model {
    pub graph: DiGraph<Node, Edge>,
    groups_map: HashMap<u64, Group>
}

impl Model {

    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let mut graph = DiGraph::new();

        let footpath_maps = csv_reader::read_to_maps(&format!("{}footpaths.csv", csv_folder_path));
        let station_maps = csv_reader::read_to_maps(&format!("{}stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}trips.csv", csv_folder_path));

        // convert each list of maps into a single map with multiple entries with id as key
        let footpaths_vec = footpath::Footpath::from_maps_to_vec(&footpath_maps);
        let mut stations_map = station::Station::from_maps_to_map(&station_maps);
        let trips_map = trip::Trip::from_maps_to_map(&trip_maps);

        // iterate over all trips
        for (_, trip) in trips_map.iter() {

            // ARRIVAL NODE
            let arrival_node_index = graph.add_node(Node {
                station: trip.to_station.clone(),
                trip_id: trip.id,
                time: trip.arrival,
                kind: NodeType::Arrival
            });

            // DEPARTURE NODE
            let departure_node_index = graph.add_node(Node {
                station: trip.from_station.clone(),
                trip_id: trip.id,
                time: trip.departure,
                kind: NodeType::Departure
            });

            // add these nodes to a station
            let to_station = stations_map.get_mut(&trip.to_station).unwrap();
            to_station.arrival_node_indices.insert(trip.id, arrival_node_index);

            let from_station = stations_map.get_mut(&trip.from_station).unwrap();
            from_station.departure_node_indices.insert(trip.id, departure_node_index);

            // connect stations of this trip
            graph.add_edge(departure_node_index, arrival_node_index, Edge::Trip {
                capacity: trip.capacity,
                duration: trip.arrival - trip.departure
            });
        }


        // iterate over all stations
        for (station_id, station) in stations_map.iter_mut() {

            // iterate over all departures
            for (trip_id, departure_node_index) in station.departure_node_indices.iter() {

                let departure_node = graph.node_weight(*departure_node_index).unwrap();
                let departure_node_time = departure_node.time;

                // DEPARTURE TRANSFER NODE (each departure also induces a corresponding departure node at the station)
                let departure_transfer_node_index = graph.add_node(Node {
                    station: station_id.clone(),
                    trip_id: *trip_id,
                    time: departure_node_time,
                    kind: NodeType::Transfer
                });

                // edge between transfer of this station to departure
                graph.add_edge(departure_transfer_node_index, *departure_node_index, Edge::Embark{});

                // add transfer node to list of transfer nodes of this station
                station.transfer_node_indices.push((departure_node_time, departure_transfer_node_index));

                // connect arrival of this trip to departure of this trip (if exists)
                // this edge represents staying in the same train
                match station.arrival_node_indices.get(trip_id) {
                    Some(arrival_node_index) => {
                        let arrival_node_time = graph.node_weight(*arrival_node_index).unwrap().time;

                        graph.add_edge(*arrival_node_index, *departure_node_index, Edge::Stay {
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
                graph.add_edge(transfer_node_indices[0].1, transfer_node_indices[1].1, Edge::TransferWait {
                    duration: transfer_node_indices[1].0 - transfer_node_indices[0].0
                });
            }

            // iterate over all arrivals
            for (_, arrival_node_index) in station.arrival_node_indices.iter() {
                
                let arrival_node = graph.node_weight(*arrival_node_index).expect("Could not find node in graph");
                let arrival_node_time = arrival_node.time;

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
                let arrival_node_time = graph.node_weight(*arrival_node_index).unwrap().time;

                // timestamp of arrival at the footpaths to_station
                let earliest_transfer_time = arrival_node_time + footpath.duration;

                // try to find next transfer node at to_station
                for (transfer_timestamp, transfer_node_index) in to_station.transfer_node_indices.iter() {
                    if earliest_transfer_time <= *transfer_timestamp {
                        graph.add_edge(*arrival_node_index, *transfer_node_index, Edge::TransferWalk {
                            duration: footpath.duration
                        });
                        break // the loop
                    }
                }

                println!("There couldn't be found any valid (time) transfer node for footpath from {} -> {}", footpath.from_station, footpath.to_station);
            }
        }


        let group_maps = csv_reader::read_to_maps(&format!("{}groups.csv", csv_folder_path));
        Self {
            graph,
            groups_map: group::Group::from_maps_to_map(&group_maps)
        }
    }


}