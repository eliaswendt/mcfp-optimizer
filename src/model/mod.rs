pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;

use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::algo::{dijkstra, min_spanning_tree};
use petgraph::data::FromElements;
use petgraph::dot::{Dot, Config};

use std::collections::{HashMap};

use crate::csv_reader;

#[derive(Debug)]
enum NodeType {
    Departure,
    Arrival,
    Transfer,
}

pub struct Node {
    id: String,

    time: u64, // time of arrival/departure
    kind: NodeType // type of this node (departure, arrival or stay)
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
         .field("id", &self.id)
         .finish()
    }
}

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

/// represents a connection between to stations or a station itself
#[derive(Debug)]
pub struct Edge {
    capacity: u64, // number of passengers this connection has capacity for
    duration: u64 // number of minutes required to get from node to node along this edge
}




/// entire combined data model
pub struct Model {
    pub graph: DiGraph<Node, Edge>,
}

impl Model {
    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let mut graph = DiGraph::new();

        let footpath_maps = csv_reader::read_to_maps(&format!("{}footpaths.csv", csv_folder_path));
        let group_maps = csv_reader::read_to_maps(&format!("{}groups.csv", csv_folder_path));
        let station_maps = csv_reader::read_to_maps(&format!("{}stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}trips.csv", csv_folder_path));

        // convert each list of maps into a single map with multiple entries with id as key
        let groups_map = group::Group::from_maps_to_map(&group_maps);
        let mut stations_map = station::Station::from_maps_to_map(&station_maps);
        let trips_map = trip::Trip::from_maps_to_map(&trip_maps);


        // parse trips that will connect all the stations
        for (_, trip) in trips_map.iter() {

            // add nodes for departure and arrival of this trip
            let departure_node_key = format!("{}_departure_{}", trip.from_station, trip.id);

            // DEPARTURE NODE
            let departure_node_index = graph.add_node(Node {
                id: departure_node_key.clone(),
                time: trip.departure,
                kind: NodeType::Departure
            });

            // TRANSFER NODE (each departure also induces a corresponding departure node at the station)
            let transfer_node_index = graph.add_node(Node {
                id: format!("{}_transfer", departure_node_key),
                time: trip.departure,
                kind: NodeType::Transfer
            });

            // ARRIVAL NODE
            let arrival_node_index = graph.add_node(Node {
                id: format!("{}_arrival_{}", trip.to_station, trip.id),
                time: trip.arrival,
                kind: NodeType::Arrival
            });

            // now add these nodes to a station
            let to_station = stations_map.get_mut(&trip.to_station).unwrap();
            to_station.arrival_node_indices.insert(trip.id, arrival_node_index);

            let from_station = stations_map.get_mut(&trip.from_station).unwrap();
            from_station.departure_node_indices.insert(trip.id, departure_node_index);
            from_station.transfer_node_indices.insert(trip.id,transfer_node_index);

            graph.add_edge(departure_node_index, arrival_node_index, Edge {
                capacity: trip.capacity,
                duration: trip.arrival - trip.departure
            });
        }


        for (_, station) in stations_map.iter() {
            station.add_connections(&mut graph);
        }

        Self {
            graph
        }
    }
}