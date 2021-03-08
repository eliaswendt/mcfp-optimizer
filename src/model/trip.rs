use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use petgraph::graph::DiGraph;

use super::{station::Station, TimetableEdge, TimetableNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: u64,
    pub from_station: String,
    pub departure: u64,
    pub to_station: String,
    pub arrival: u64,
    pub capacity: u64,
}

impl Trip {
    pub fn from_maps_to_vec(trip_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {
        println!("parsing {} trip(s)", trip_maps.len());

        let mut trips = Vec::with_capacity(trip_maps.len());

        for trip_map in trip_maps.iter() {
            let id = trip_map.get("id").unwrap().parse().unwrap();
            let from_station = trip_map.get("from_station").unwrap().clone();
            let to_station = trip_map.get("to_station").unwrap().clone();

            println!("{}_{}->{}", id, from_station, to_station);

            trips.push(Self {
                id,
                from_station,
                departure: trip_map.get("departure").unwrap().parse().unwrap(),
                to_station,
                arrival: trip_map.get("arrival").unwrap().parse().unwrap(),
                capacity: trip_map.get("capacity").unwrap().parse().unwrap(),
            });
        }

        trips
    }

    pub fn connect(
        self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        stations: &mut HashMap<String, Station>,
    ) {
        let from_station = stations.get_mut(&self.from_station).expect(&format!(
            "from_station {} of trip {} could not be found",
            &self.from_station, self.id
        ));
        let departure = from_station.add_departure(graph, self.id, self.departure);

        let to_station = stations.get_mut(&self.to_station).expect(&format!(
            "to_station {} of trip {} could not be found",
            &self.to_station, self.id
        ));
        let arrival = to_station.add_arrival(graph, self.id, self.arrival);

        // connect start and end of this ride
        graph.add_edge(
            departure,
            arrival,
            TimetableEdge::Trip {
                duration: self.arrival - self.departure,
                capacity_soft_limit: (self.capacity as f64 * 0.75) as u64,
                capacity_hard_limit: self.capacity,
                utilization: 0,
            },
        );
    }
}
