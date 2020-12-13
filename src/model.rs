use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::algo::{dijkstra, min_spanning_tree};
use petgraph::data::FromElements;
use petgraph::dot::{Dot, Config};

use std::collections::{HashMap};

use crate::csv_reader;

struct Footpath {
    from_station: String,
    to_station: String,
    duration: u64
}

impl Footpath {
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self {
            from_station: map.get("from_station").unwrap().clone(),
            to_station: map.get("to_station").unwrap().clone(),
            duration: map.get("duration").unwrap().parse().unwrap()
        }
    }
}

struct Station {
    id: String,
    transfer: u64, // transfer time (minutes) at this station
    name: String, 
}

impl Station {
    pub fn from_maps_to_map(station_maps: &Vec<HashMap<String, String>>) -> HashMap<String, Self> {

        println!("parsing {} stations", station_maps.len());

        let mut stations_map = HashMap::with_capacity(station_maps.len());

        for station_map in station_maps.iter() {
            let id = station_map.get("id").unwrap().clone();

            stations_map.insert(id.clone(), Self {
                id,
                transfer: station_map.get("transfer").unwrap().parse().unwrap(),
                name: station_map.get("name").unwrap().clone(),
            });
        }

        stations_map
    }
}

struct Trip {
    id: u64,
    from_station: String,
    departure: u64,
    to_station: String,
    arrival: u64,
    capacity: u64
}

impl Trip {
    pub fn from_maps_to_map(trip_maps: &Vec<HashMap<String, String>>) -> HashMap<u64, Self> {

        println!("parsing {} trips", trip_maps.len());

        let mut trips_map = HashMap::with_capacity(trip_maps.len());

        for trip_map in trip_maps.iter() {

            let id = trip_map.get("id").unwrap().parse().unwrap();

            trips_map.insert(id, 
                Self {
                id,
                from_station: trip_map.get("from_station").unwrap().clone(),
                departure: trip_map.get("departure").unwrap().parse().unwrap(),
                to_station: trip_map.get("to_station").unwrap().clone(),
                arrival: trip_map.get("arrival").unwrap().parse().unwrap(),
                capacity: trip_map.get("capacity").unwrap().parse().unwrap()
            });
        }

        trips_map
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

impl Group {
    pub fn from_maps_to_map(group_maps: &Vec<HashMap<String, String>>) -> HashMap<u64, Self> {

        println!("parsing {} groups", group_maps.len());

        let mut groups_map = HashMap::with_capacity(group_maps.len());

        for group_map in group_maps.iter() {
            let id = group_map.get("id").unwrap().parse().unwrap();

            let in_trip_value = group_map.get("in_trip").unwrap();
            let in_trip = if in_trip_value.is_empty() {
                None
            } else {
                Some(in_trip_value.parse().unwrap())
            };

            groups_map.insert(id, Self {
                id,
                start: group_map.get("start").unwrap().clone(),
                destination: group_map.get("destination").unwrap().clone(),
                departure: group_map.get("departure").unwrap().parse().unwrap(),
                arrival: group_map.get("arrival").unwrap().parse().unwrap(),
                passengers: group_map.get("passengers").unwrap().parse().unwrap(),
                in_trip
            });
        } 

        groups_map
    }
}


enum Node {
    Arrival {
        time: u64,
    },

    Departure {
        time: u64
    },

    Transfer {
        duration: u64 // transfer time at this station
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
    duration: u64 // number of minutes required to get from node to node along this edge
}



/// entire combined data model
pub struct Model {
    graph: DiGraph<Node, Edge>,
}

impl Model {
    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let mut graph = DiGraph::new();

        let footpath_maps = csv_reader::read_to_maps(&format!("{}footpaths.csv", csv_folder_path));
        let group_maps = csv_reader::read_to_maps(&format!("{}groups.csv", csv_folder_path));
        let station_maps = csv_reader::read_to_maps(&format!("{}stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}trips.csv", csv_folder_path));

        // convert each list of maps into a single map with multiple entries with id as key
        let groups_map = Group::from_maps_to_map(&group_maps);
        let stations_map = Station::from_maps_to_map(&station_maps);
        let trips_map = Trip::from_maps_to_map(&trip_maps);


        // parse trips that will connect all the stations
        for (trip_id, trip) in trips_map.iter() {

            // add nodes for start and destination of this trip
            let node_departure = graph.add_node(Node::new_departure(trip.departure));
            let node_arrival = graph.add_node(Node::new_arrival(trip.arrival));

            // add edge between those two nodes
            graph.add_edge(node_departure, node_arrival, Edge {
                capacity: trip.capacity,
                duration: (trip.arrival - trip.departure)
            });

            let from_station = stations_map.get(&trip.from_station).unwrap();
            let to_station = stations_map.get(&trip.to_station).unwrap();


        }




        // finally build actual graph from stations





        Self {
            graph
        }
    }
}