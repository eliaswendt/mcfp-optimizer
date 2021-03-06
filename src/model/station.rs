use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

use super::{TimetableEdge, TimetableNode};

/// stop station 
pub struct Station {
    pub id: u64, // unique identifer
    pub transfer_time: u64, // transfer time (minutes) at this station
    pub name: String, // station's name

    // key is the trip_id, value is Vec<>, because one trip may have multiple arrivals/departures at the same station
    pub arrivals: HashMap<u64, Vec<NodeIndex>>,
    pub departures: HashMap<u64, Vec<NodeIndex>>,

    pub transfers: Vec<NodeIndex>,
}

impl Station {

    /// returns stations from maps
    pub fn from_maps_to_map(station_maps: &Vec<HashMap<String, String>>) -> HashMap<u64, Self> {
        println!("parsing {} station(s)", station_maps.len());

        let mut stations_map = HashMap::with_capacity(station_maps.len());

        for station_map in station_maps.iter() {
            let id = station_map.get("id").unwrap().parse().expect("Could not parse station id!");
            let name = station_map.get("name").unwrap().clone();

            stations_map.insert(
                id,
                Self {
                    id: id,
                    transfer_time: station_map.get("transfer").unwrap().parse().unwrap(),
                    name: name.clone(),

                    arrivals: HashMap::new(),
                    departures: HashMap::new(),
                    transfers: Vec::new(),
                },
            );
        }

        stations_map
    }

    /// adds departure node to graph
    pub fn add_departure(
        &mut self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        trip_id: u64,
        time: u64,
    ) -> NodeIndex {
        // create departure node
        let departure = graph.add_node(TimetableNode::Departure {
            trip_id,
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone(),
        });

        // if trip_id does not exist -> create new vec, then push arrival to the end of the list
        self.departures
            .entry(trip_id)
            .or_insert(Vec::new())
            .push(departure);

        // create transfer node, as each departure also induces a corresponding transfer node at the station
        let transfer = graph.add_node(TimetableNode::Transfer {
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone(),
        });

        // add edge between transfer of this station to departure
        graph.add_edge(transfer, departure, TimetableEdge::Board);

        // add transfer node to list of transfer nodes of this station
        self.transfers.push(transfer);

        departure
    }

    /// adds arrival node to graph
    pub fn add_arrival(
        &mut self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        trip_id: u64,
        time: u64,
    ) -> NodeIndex {
        // create node
        let arrival = graph.add_node(TimetableNode::Arrival {
            trip_id,
            time,
            station_id: self.id.clone(),
            station_name: self.name.clone(),
        });

        // if key does not exist -> create new vec, then push arrival to the end of the list
        self.arrivals
            .entry(trip_id)
            .or_insert(Vec::new())
            .push(arrival);

        arrival
    }

    /// connects all arrival-/transfer-/departure nodes of this station with each other
    ///
    /// consumes self to prevent programmers to add departures or arrivals afterwards ;)
    ///
    /// returns the transfer nodes (for departure) and the main arrival nodes
    pub fn connect(
        mut self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    ) -> (Vec<NodeIndex>, Vec<NodeIndex>) {

        // FIRST: sort transfers list by time (first tuple element)
        self.transfers.sort_unstable_by_key(|transfer| graph[*transfer].time());

        // SECOND: connect all transfers with each other (only pairwise)
        for transfer_slice in self.transfers.windows(2) {

            let transfer_a_time = graph[transfer_slice[0]].time();
            let transfer_b_time = graph[transfer_slice[1]].time();

            graph.add_edge(
                transfer_slice[0],
                transfer_slice[1],
                TimetableEdge::WaitAtStation {
                    duration: transfer_b_time - transfer_a_time,
                },
            );
        }

        // THIRD: iterate over all arrivals and connect them to the station's next available transfer
        for arrival in self.arrivals.values().flatten() {
            let arrival_time = graph[*arrival].time();
            let earliest_transfer_time = arrival_time + self.transfer_time;

            // try to find next transfer node at this station (requires transfers to be sorted (earliest first))
            for transfer in self.transfers.iter() {
                if earliest_transfer_time <= graph[*transfer].time() {
                    graph.add_edge(
                        *arrival,
                        *transfer,
                        TimetableEdge::Alight {
                            duration: self.transfer_time,
                        },
                    );
                    break; // we connected a reachable transfer node -> break search loop
                }
            }
        }

        // FOURTH: connect arrival of this trip to departure of this trip (if exists)
        for (trip_id, arrivals_of_trip) in self.arrivals.iter() {
            let departures_of_trip = match self.departures.get(trip_id) {
                Some(departure) => departure,
                None => continue, // with next arrival
            };

            // from here on we have two vecs of arrivals and departures of the same trip
            for (arrival, departure) in arrivals_of_trip.iter().zip(departures_of_trip.iter()) {
                let arrival_time = graph[*arrival].time();
                let departure_time = graph[*departure].time();

                // only create edge between arrival and departure only if arrival is before (time) departure
                // this is required, as it otherwise would also connect start-/end station of a trip with equal start/destination
                if arrival_time <= departure_time {
                    graph.add_edge(
                        *arrival,
                        *departure,
                        TimetableEdge::WaitInTrain {
                            duration: departure_time - arrival_time,
                        },
                    );
                }
            }
        }

        // return transfer and arrival node indices (without time/trip_id)
        (
            self.transfers,
            self.arrivals.values().flatten().cloned().collect(),
        )
    }
}
