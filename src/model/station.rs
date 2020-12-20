use std::collections::{HashMap, LinkedList};
use petgraph::graph::{DiGraph,NodeIndex};
pub struct Station {
    id: String,
    pub transfer_time: u64, // transfer time (minutes) at this station
    pub name: String, 

    pub arrival_node_indices: HashMap<u64, NodeIndex>,
    pub departure_node_indices: HashMap<u64, NodeIndex>,
    pub transfer_node_indices: Vec<(u64, NodeIndex)>, // key is departure time
}

impl Station {
    pub fn from_maps_to_map(station_maps: &Vec<HashMap<String, String>>) -> HashMap<String, Self> {

        println!("parsing {} stations", station_maps.len());

        let mut stations_map = HashMap::with_capacity(station_maps.len());

        for station_map in station_maps.iter() {
            let id = station_map.get("id").unwrap().clone();

            stations_map.insert(id.clone(), Self {
                id,
                transfer_time: station_map.get("transfer").unwrap().parse().unwrap(),
                name: station_map.get("name").unwrap().clone(),

                arrival_node_indices: HashMap::new(),
                departure_node_indices: HashMap::new(),
                transfer_node_indices: Vec::new()
            });
        }

        stations_map
    }
}