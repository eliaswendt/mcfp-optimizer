use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};

use super::{EdgeWeight, NodeWeight};
pub struct Station {
    pub id: String,
    pub transfer_time: u64, // transfer time (minutes) at this station
    pub name: String, 

    pub arrivals: HashMap<u64, Vec<NodeIndex>>,
    pub departures: HashMap<u64, Vec<NodeIndex>>,
    pub transfers: Vec<(u64, NodeIndex)>, // key is departure time
}

impl Station {
    pub fn from_maps_to_map(station_maps: &Vec<HashMap<String, String>>, graph: &mut DiGraph<NodeWeight, EdgeWeight>) -> HashMap<String, Self> {

        println!("parsing {} station(s)", station_maps.len());

        let mut stations_map = HashMap::with_capacity(station_maps.len());

        for station_map in station_maps.iter() {
            let id = station_map.get("id").unwrap().clone();
            let name = station_map.get("name").unwrap().clone();

            stations_map.insert(id.clone(), Self {
                id: id.clone(),
                transfer_time: station_map.get("transfer").unwrap().parse().unwrap(),
                name: name.clone(),

                arrivals: HashMap::new(),
                departures: HashMap::new(),
                transfers: Vec::new()
            });
        }

        stations_map
    }


    /// add departure to station
    pub fn add_departure(
        &mut self, 
        graph: &mut DiGraph<NodeWeight, EdgeWeight>, 
        trip_id: u64,
        time: u64
    ) -> NodeIndex {

        // create departure node
        let departure = graph.add_node(NodeWeight::Departure {
            trip_id,
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone()
        });

        // if trip_id does not exist -> create new vec, then push arrival to the end of the list
        self.departures.entry(trip_id).or_insert(Vec::new()).push(departure);

        // create departure transfer node (each departure also induces a corresponding departure node at the station)
        let departure_transfer = graph.add_node(NodeWeight::Transfer {
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone()
        });

        // add edge between transfer of this station to departure
        graph.add_edge(departure_transfer, departure, EdgeWeight::Board);

        // add transfer node to list of transfer nodes of this station
        self.transfers.push(
            (time, departure_transfer)
        );

        departure
    }  

    /// add arrival to station
    pub fn add_arrival(
        &mut self, 
        graph: &mut DiGraph<NodeWeight, EdgeWeight>, 
        trip_id: u64,
        time: u64
    ) -> NodeIndex {

        // create node
        let arrival = graph.add_node(NodeWeight::Arrival {
            trip_id,
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone()
        });

        // if key does not exist -> create new vec, then push arrival to the end of the list
        self.arrivals.entry(trip_id).or_insert(Vec::new()).push(arrival);

        arrival
    }


    /// connects all arrival-/transfer-/departure nodes of this station with each other
    ///
    /// consumes self to prevent programmers to add departures or arrivals afterwards ;)
    ///
    /// returns the transfer nodes (for departure) and the main arrival nodes
    pub fn connect(
        mut self, 
        graph: &mut DiGraph<NodeWeight, EdgeWeight>
    ) -> (Vec<(u64, NodeIndex)>, Vec<NodeIndex>) {

        // FIRST: sort transfers list by time (first tuple element)
        self.transfers.sort_unstable_by_key(|(time, _)| *time);

        // SECOND: connect each transfer to the next (time)
        for transfer_slice in self.transfers.windows(2) {
            graph.add_edge(transfer_slice[0].1, transfer_slice[1].1, EdgeWeight::WaitAtStation {
                duration: transfer_slice[1].0 - transfer_slice[0].0
            });
        }

        // THIRD: iterate over all arrivals and connect them to the station's next available transfer
        for arrival in self.arrivals.values().flatten() {

            let arrival_time = graph[*arrival].get_time().unwrap();
            let earliest_transfer_time = arrival_time + self.transfer_time;

            // try to find next transfer node at this station (requires transfers to be sorted (earliest first))
            for (transfer_time, transfer) in self.transfers.iter() {

                if earliest_transfer_time <= *transfer_time {
                    graph.add_edge(*arrival, *transfer, EdgeWeight::Alight {
                        duration: self.transfer_time
                    });
                    break // the loop
                }
            }
        }

        // FOURTH: connect arrival of this trip to departure of this trip (if exists)
        for (trip_id, arrivals_of_trip) in self.arrivals.iter() {
            let departures_of_trip = match self.departures.get(trip_id) {
                Some(departure) => departure,
                None => continue // with next arrival
            };

            // from here on we have two vecs of arrivals and departures of the same trip
            for (arrival, departure) in arrivals_of_trip.iter().zip(departures_of_trip.iter()) {

                let arrival_time = graph[*arrival].get_time().unwrap();
                let departure_time = graph[*departure].get_time().unwrap();

                // only create edge between arrival and departure only if arrival is before (time) departure
                // this is required, as it otherwise would also connect start-/end station of a trip with equal start/destination
                if arrival_time <= departure_time {
                    graph.add_edge(*arrival, *departure, EdgeWeight::WaitInTrain {
                        duration: departure_time - arrival_time
                    });
                }
            }
        }

        // return transfer and arrival node indices (without time/trip_id)
        (
            self.transfers, 
            self.arrivals.values().flatten().cloned().collect()
        )
    }
}