use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::algo::{dijkstra, min_spanning_tree};
use petgraph::data::FromElements;
use petgraph::dot::{Dot, Config};

use std::collections::{HashMap};

use crate::csv_reader;

struct Station {
    id: String,
    name: String,
    transfer: usize, // transfer time (minutes) at this station

    arrivals: HashMap<u64, Node>, // incoming trips (trip.id, node)
    departures: HashMap<u64, Node>, // outgoing trips (trip.id, node)
}

impl Station {
    pub fn from_map(station_map: HashMap<String, String>) -> Self {
        Self {
            id: station_map.get("id").unwrap().clone(),
            name: station_map.get("name").unwrap().clone(),
            transfer: station_map.get("transfer").unwrap().parse().unwrap(),

            arrivals: HashMap::new(),
            departures: HashMap::new()
        }
    }
}

/// travel group
struct Group {
    id: u64,
    
    start: String, // Start-Halt für die Alternativensuche (Station ID)
    destination: String, // Ziel der Gruppe (Station ID)

    departure: usize, // Frühstmögliche Abfahrtszeit am Start-Halt (Integer)
    arrival: usize, // Ursprünglich geplante Ankunftszeit am Ziel (Integer)

    passengers: usize, // Größe der Gruppe (Integer)


    // Hier gibt es zwei Möglichkeiten (siehe auch unten):
    // Wenn der Wert leer ist, befindet sich die Gruppe am Start-Halt.
    // Wenn der Wert nicht leer ist, gibt er die Trip ID (Integer) der Fahrt an, in der sich die Gruppe befindet.
    in_trip: Option<usize>,
}


enum Node {
    Arrival {
        time: u64,
    },

    Departure {
        time: u64
    },

    Transfer {
        transfer_time: usize // transfer time at this station
    }
}

impl Node {
    pub fn new_arrival(time: u64) -> Self {
        Self::Arrival {
            time
        }
    }

    pub fn new_departure(time: u64) -> Self {
        Self::Departure {
            time
        }
    }
}

/// represents a connection between to stations or a station itself
struct Edge {
    capacity: u64, // number of passengers this connection has capacity for
    transfer_time: u64 // number of minutes required to get from node to node along this edge
}



/// entire combined data model
struct Model {
    graph: DiGraph<Node, Edge>,
}

impl Model {
    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let mut graph = DiGraph::new();

        let station_maps = csv_reader::read_to_maps(&format!("{}/stations.csv", csv_folder_path));
        let footpath_maps = csv_reader::read_to_maps(&format!("{}/footpaths.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}/trips.csv", csv_folder_path));

        // first: parse maps of stations (rows from csv) into a single map with stations (with station.id as key)
        let mut stations_map: HashMap<String, Station> = HashMap::with_capacity(station_maps.len());
        for station_map in station_maps.into_iter() {

            let station = Station::from_map(station_map);
            stations_map.insert(station.id.clone(), station);
        }

        // second: parse trips that will connect all the stations
        for trip_map in trip_maps.into_iter() {

            // parse departure and arrival time
            let departure: u64 = trip_map.get("departure").unwrap().parse().unwrap();
            let arrival: u64 = trip_map.get("arrival").unwrap().parse().unwrap();

            // add nodes for start and destination of this trip
            let node_departure = graph.add_node(Node::new_departure(departure));
            let node_arrival = graph.add_node(Node::new_arrival(arrival));

            // add edge between those two nodes
            graph.add_edge(node_departure, node_arrival, Edge {
                capacity: trip_map.get("capacity").unwrap().parse().unwrap(),
                transfer_time: (arrival - departure)
            });


            let from_station_id = trip_map.get("from_station").unwrap();
            let to_station_id = trip_map.get("to_station").unwrap();

            let from_station = stations_map.get(from_station_id).unwrap();
            let to_station = stations_map.get(to_station_id).unwrap();


        }




        // finally build actual graph from stations





        Self {
            graph
        }
    }
}