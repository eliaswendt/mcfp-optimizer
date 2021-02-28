use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};

use super::{TimetableEdge, TimetableNode};

pub struct Footpath {
    pub from_station: String,
    pub to_station: String,
    pub duration: u64,
}

impl Footpath {
    pub fn from_maps_to_vec(footpath_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {
        println!("parsing {} footpath(s)", footpath_maps.len());

        let mut footpaths_vec = Vec::with_capacity(footpath_maps.len());

        for footpath_map in footpath_maps.iter() {
            footpaths_vec.push(Self {
                from_station: footpath_map.get("from_station").unwrap().clone(),
                to_station: footpath_map.get("to_station").unwrap().clone(),
                duration: footpath_map.get("duration").unwrap().parse().unwrap(),
            });
        }

        footpaths_vec
    }

    pub fn connect(
        self,
        graph: &mut DiGraph<TimetableNode, TimetableEdge>,
        from_station_arrivals: &Vec<NodeIndex>,
        to_station_transfers: &Vec<(u64, NodeIndex)>,
    ) -> (u64, u64) {
        let mut successful_footpath_counter = 0;
        let mut failed_footpath_counter = 0;

        // for every arrival at the from_station try to find the next transfer node at the to_station
        for arrival in from_station_arrivals.iter() {
            let arrival_time = graph[*arrival].get_time().unwrap();

            // timestamp of arrival at the footpaths to_station
            let earliest_transfer_time = arrival_time + self.duration;

            let mut edge_added = false;

            // try to find next transfer node at to_station (requires transfers to be sorted, earliest first)
            for (transfer_time, transfer) in to_station_transfers.iter() {
                if earliest_transfer_time <= *transfer_time {
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
