use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use petgraph::graph::DiGraph;

use super::{station::Station, TimetableEdge, TimetableNode};

/// a trip from a station to another station
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trip {
    pub id: u64, // trip id
    pub from_station: u64, // station to departure
    pub departure: u64, // departure time
    pub to_station: u64, // station to a arrive
    pub arrival: u64, // arrival time
    pub capacity: u64, // capacity on this trip 
}

impl Trip {

    /// returns trips from maps
    pub fn from_maps_to_vec(trip_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {
        println!("parsing {} trip(s)", trip_maps.len());

        let mut trips = Vec::with_capacity(trip_maps.len());

        for trip_map in trip_maps.iter() {
            let id = trip_map.get("id").unwrap().parse().unwrap();
            let from_station = trip_map.get("from_station").unwrap().parse().unwrap();
            let to_station = trip_map.get("to_station").unwrap().parse().unwrap();

            // println!("{}_{}->{}", id, from_station, to_station);

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

    /// connects a departure node with an arrival node with a trip edge
    pub fn connect(
        self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        stations: &mut HashMap<u64, Station>,
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
                capacity: self.capacity,
                utilization: 0,
            },
        );
    }
}
