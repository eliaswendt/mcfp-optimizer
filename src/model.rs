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
    pub fn from_maps_to_map(trip_maps: &Vec<HashMap<String, String>>) -> HashMap<String, Self> {

        println!("parsing {} trips", trip_maps.len());

        let mut trips_map = HashMap::with_capacity(trip_maps.len());

        for trip_map in trip_maps.iter() {

            let id = trip_map.get("id").unwrap().parse().unwrap();
            let from_station = trip_map.get("from_station").unwrap().clone();
            let to_station = trip_map.get("to_station").unwrap().clone();

            let key = format!("{}_{}->{}", id, from_station, to_station);

            trips_map.insert(key, 
                Self {
                id,
                from_station: from_station,
                departure: trip_map.get("departure").unwrap().parse().unwrap(),
                to_station: to_station,
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

#[derive(Debug)]
enum StationType {
    Departure,
    Arrival,
    Transfer,
}

#[derive(Debug)]
pub struct Node {
    id: String,

    time: u64, // time of arrival/departure
    kind: StationType // type of this node (departure, arrival or stay)
}

impl Node {
    pub fn is_arrival(&self) -> bool {
        match self.kind {
            StationType::Arrival => true,
            _ => false
        }
    }

    pub fn is_departure(&self) -> bool {
        match self.kind {
            StationType::Departure => true,
            _ => false
        }
    }

    pub fn is_transfer(&self) -> bool {
        match self.kind {
            StationType::Transfer => true,
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



struct StationHelper {
    arrivals: HashMap<String, NodeIndex>,
    departures: HashMap<String, NodeIndex>,
    transfers: HashMap<u64, NodeIndex>, // key is time!

    transfer_time: u64,
}

impl StationHelper {
    pub fn new(transfer_time: u64) -> Self {
        StationHelper {
            arrivals: HashMap::new(),
            departures: HashMap::new(),
            transfers: HashMap::new(),

            transfer_time
        }
    }

    /// connect arrivals with departures, arrivals with transfers, transfers with transfers and transfers with departures
    pub fn connect(&self) {

    }


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
        let groups_map = Group::from_maps_to_map(&group_maps);
        let stations_map = Station::from_maps_to_map(&station_maps);
        let trips_map = Trip::from_maps_to_map(&trip_maps);

        // store node indices so that we do not have to search them iin the graph afterwards
        let mut arrival_node_indices: HashMap<String, NodeIndex> = HashMap::with_capacity(trips_map.len());
        let mut departure_node_indices: HashMap<String, NodeIndex> = HashMap::with_capacity(trips_map.len());

        // parse trips that will connect all the stations
        for (_, trip) in trips_map.iter() {

            let departure_node_key = format!("{}_departure_{}", trip.from_station, trip.id);
            let transfer_node_key = format!("transfer_{}", departure_node_key);
            let arrival_node_key =  format!("{}_arrival_{}", trip.to_station, trip.id);

            // add nodes for departure and arrival of this trip

            // DEPARTURE NODE
            let departure_node_index = graph.add_node(Node {
                id: departure_node_key.clone(),
                time: trip.departure,
                kind: StationType::Departure
            });

            // TRANSFER NODE (each departure also induces a corresponding departure node at the station)
            let transfer_node_index = graph.add_node(Node {
                id: transfer_node_key.clone(),
                time: trip.departure,
                kind: StationType::Transfer
            });

            // ARRIVAL NODE
            let arrival_node_index = graph.add_node(Node {
                id: arrival_node_key.clone(),
                time: trip.arrival,
                kind: StationType::Arrival
            });

            // add edge between departure and arrival
            graph.add_edge(departure_node_index, arrival_node_index, Edge {
                capacity: trip.capacity,
                duration: (trip.arrival - trip.departure)
            });

            // add edge between transfer and departure
            graph.add_edge(transfer_node_index, departure_node_index, Edge {
                capacity: u64::MAX,
                duration: 0
            });

            departure_node_indices.insert(departure_node_key, departure_node_index);
            arrival_node_indices.insert(arrival_node_key, arrival_node_index);
        }

        // iterate again, but this time we want to connect the arrivals with their departures
        for (_, trip) in trips_map.iter() {
            let station_id = &trip.from_station;

            // we now connect arrival with the departure at this trip's departure station
            let departure_node_key = format!("{}_departure_{}", station_id, trip.id);
            let arrival_node_key =  format!("{}_arrival_{}", station_id, trip.id);



            // now try to find arrival and departure nodes in the HashMaps we filled in the previous loop
            let arrival_node_index = match arrival_node_indices.get(&arrival_node_key) {
                Some(arrival_node) => *arrival_node,
                None => continue // with next trip
            };

            let departure_node_index = match departure_node_indices.get(&departure_node_key) {
                Some(departure_node) => *departure_node,
                None => continue // with next trip
            };

            println!("connecting {} <-> {}", arrival_node_key, departure_node_key);

            graph.add_edge(arrival_node_index, departure_node_index, Edge {
                capacity: u64::MAX,
                duration: 0
            });
        }



        // finally build actual graph from stations





        Self {
            graph
        }
    }
}