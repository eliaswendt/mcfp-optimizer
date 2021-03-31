use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};

use super::{TimetableEdge, TimetableNode};

/// footpath from a station to another station
pub struct Footpath {
    pub from_station: u64,
    pub to_station: u64,
    pub duration: u64,
}

impl Footpath {

    /// returns footpaths from maps
    pub fn from_maps_to_vec(footpath_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {
        println!("parsing {} footpath(s)", footpath_maps.len());

        let mut footpaths_vec = Vec::with_capacity(footpath_maps.len());

        for footpath_map in footpath_maps.iter() {
            footpaths_vec.push(Self {
                from_station: footpath_map.get("from_station").unwrap().parse().unwrap(),
                to_station: footpath_map.get("to_station").unwrap().parse().unwrap(),
                duration: footpath_map.get("duration").unwrap().parse().unwrap(),
            });
        }

        footpaths_vec
    }

    /// connects all arrivals of a station with the earliest-reachable transfers at the footpath's destination station
    pub fn connect(
        self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        from_station_arrivals: &Vec<NodeIndex>,
        to_station_transfers: &Vec<NodeIndex>,
    ) -> (u64, u64) {
        let mut successful_footpath_counter = 0;
        let mut failed_footpath_counter = 0;

        // for every arrival at the from_station try to find the next transfer node at the to_station
        for arrival in from_station_arrivals.iter() {
            let arrival_time = graph[*arrival].time();

            // timestamp of arrival at the footpaths to_station
            let earliest_transfer_time = arrival_time + self.duration;

            let mut edge_added = false;

            // try to find next transfer node at to_station (requires transfers to be sorted, earliest first)
            for transfer in to_station_transfers.iter() {
                if earliest_transfer_time <= graph[*transfer].time() {
                    graph.add_edge(
                        *arrival,
                        *transfer,
                        TimetableEdge::Walk {
                            duration: self.duration,
                        },
                    );
                    edge_added = true;
                    successful_footpath_counter += 1;
                    break; // the inner loop
                }
            }

            if !edge_added {
                failed_footpath_counter += 1;
                //println!("There couldn't be found any valid (time) transfer node for footpath from {} -> {}", footpath.from_station, footpath.to_station);
            }
        }

        (successful_footpath_counter, failed_footpath_counter)
    }
}
