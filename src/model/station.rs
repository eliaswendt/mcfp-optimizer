use std::collections::{HashMap, LinkedList};
use petgraph::graph::{DiGraph,NodeIndex};
pub struct Station {
    id: String,
    transfer: u64, // transfer time (minutes) at this station
    name: String, 

    pub arrival_node_indices: HashMap<u64, NodeIndex>,
    pub departure_node_indices: HashMap<u64, NodeIndex>,
    pub transfer_node_indices: HashMap<u64, NodeIndex>, // key is trip.id of corresponding departure // todo: replace key with departure time
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

                arrival_node_indices: HashMap::new(),
                departure_node_indices: HashMap::new(),
                transfer_node_indices: HashMap::new()
            });
        }

        stations_map
    }

    /// create a representation of this station data structure into the 
    pub fn add_connections(&self, graph: &mut DiGraph<super::Node, super::Edge>) {

        // iterate over all departures
        for (trip_id, departure_node_index) in self.departure_node_indices.iter() {

            // try to connect departure node to corresponding arrival node
            match self.arrival_node_indices.get(trip_id) {
                Some(arrival_node_index) => {
                    // sitzen bleiben im zug
                    graph.add_edge(*arrival_node_index, *departure_node_index, super::Edge {
                        capacity: u64::MAX,
                        duration: 0 // todo: consider stay time
                    });
                }
                None => {}
            }

            // try to connect transfer node to 
            match self.transfer_node_indices.get(trip_id) {
                Some(transfer_node_index) => {
                    graph.add_edge(*transfer_node_index, *departure_node_index, super::Edge {
                        capacity: u64::MAX,
                        duration: 0
                    });
                }
                None => {}
            }
        }

        let mut transfer_node_indices_list: Vec<(&u64, &NodeIndex)> = self.transfer_node_indices.iter().collect();
        transfer_node_indices_list.sort_unstable_by_key(|(key, _)| **key);

        // connect transfers with each other
        // todo: do not run in every iteration
        for transfer_node_indices in transfer_node_indices_list.windows(2) {
            graph.add_edge(
                *transfer_node_indices[0].1,
                *transfer_node_indices[1].1, 
                super::Edge {
                    capacity: u64::MAX,
                    duration: 0
                }
            );
        }

        // connect arrival nodes to the station's transfer nodes
        for (arrival_node_key, arrival_node_index) in self.arrival_node_indices.iter() {
            let earliest_transfer_time = graph.node_weight(*arrival_node_index).unwrap().time + self.transfer;

            // find next transfer node after earliest_transfer_time
            for (transfer_node_time, transfer_node_index) in transfer_node_indices_list.iter() {
                if **transfer_node_time >= earliest_transfer_time {
                    graph.add_edge(*arrival_node_index, **transfer_node_index, super::Edge {
                        capacity: u64::MAX,
                        duration: self.transfer
                    });
                    break;
                }
            }
        }
    }
}