use std::{collections::HashMap, time::Instant};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::io::Read;

use colored::Colorize;
use petgraph::graph::{DiGraph, EdgeIndex};

use super::{Model, TimetableEdge, TimetableNode, path::{self, Path}};

/// travel group
#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    
    pub start: String, // Start-Halt für die Alternativensuche (Station ID)
    pub destination: String, // Ziel der Gruppe (Station ID)

    pub departure: u64, // Frühstmögliche Abfahrtszeit am Start-Halt (Integer)
    pub arrival: u64, // Ursprünglich geplante Ankunftszeit am Ziel (Integer)

    pub passengers: usize, // Größe der Gruppe (Integer)

    // Hier gibt es zwei Möglichkeiten (siehe auch unten):
    // Wenn der Wert leer ist, befindet sich die Gruppe am Start-Halt.
    // Wenn der Wert nicht leer ist, gibt er die Trip ID (Integer) der Fahrt an, in der sich die Gruppe befindet.
    pub in_trip: Option<usize>,

    pub paths: Vec<Path> // possible paths for this group
}

impl Group {
    pub fn from_maps_to_vec(group_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {

        println!("parsing {} group(s)", group_maps.len());

        let mut groups = Vec::with_capacity(group_maps.len());

        for group_map in group_maps.iter() {
            let id = group_map.get("id").unwrap().parse().unwrap();

            let in_trip_value = group_map.get("in_trip").unwrap();
            let in_trip = if in_trip_value.is_empty() {
                None
            } else {
                Some(in_trip_value.parse().unwrap())
            };

            groups.push(Self {
                id,
                start: group_map.get("start").unwrap().clone(),
                destination: group_map.get("destination").unwrap().clone(),
                departure: group_map.get("departure").unwrap().parse().unwrap(),
                arrival: group_map.get("arrival").unwrap().parse().unwrap(),
                passengers: group_map.get("passengers").unwrap().parse().unwrap(),
                in_trip,
                paths: Vec::new()
            });
        } 

        groups
    }


    /// returns (remaining_duration, path), returns true if there was at least one path found
    pub fn search_paths(&mut self, model: &Model, max_budget: u64, duration_factor: f64) -> bool {

        let from = model.find_start_node_index(&self.start, self.departure).expect("Could not find departure at from_station");
        let to = model.find_end_node_index(&self.destination).expect("Could not find destination station");

        // max duration should depend on the original travel time
        let travel_time = self.arrival - self.departure;
        let max_duration = (travel_time as f64 * duration_factor) as u64; // todo: factor to modify later if not a path could be found for all groups

        let start = Instant::now();
        print!("{} -> {} with {} passenger(s) in {} min(s) ... ", self.start, self.destination, self.passengers, max_duration);
        // self.paths = path::Path::search_recursive_dfs(
        //     &model.graph, 
        //     from,
        //     to, //|node| node.is_arrival_at_station(&group_value.destination), // dynamic condition for dfs algorithm to find arrival node

        //     self.passengers as u64, 
        //     max_duration, 
        //     max_budget // initial budget for cost (each edge has individual search cost)
        // );
        self.paths = path::Path::all_paths_iddfs(
            &model.graph,
            from,
            to,
            self.passengers as u64, 
            max_duration,
            5,
            50,
            100,
        );

        print!("done in {}ms, ", start.elapsed().as_millis());

        // sort by remaining_duration (highest first)
        self.paths.sort_unstable();
        self.paths.reverse();

        if self.paths.len() == 0 {
            println!("{}", format!("no path found").red());
            false
        } else {
            println!("{}", format!("{} paths, best={{duration={}, len={}}}", self.paths.len(), self.paths[0].duration(), self.paths[0].len()).green());
            true
        }
    }

    pub fn dump_groups(groups: Vec<Group>, group_folder_path: &str) {
        println!("Dumping groups...");
        let serialized_groups = serde_json::to_string(&groups).unwrap();
        let mut file = std::fs::File::create(&format!("{}groups.json", group_folder_path)).expect("File creation failed!");
        file.write_all(serialized_groups.as_bytes()).expect("Could not write graph in file!")
    }

    pub fn load_groups(group_folder_path: &str) -> Vec<Group> {
        println!("Loading groups...");
        let mut file = std::fs::File::open(&format!("{}groups.json", group_folder_path)).expect("File opening failed!");
        let mut serialized_groups = String::new();
        file.read_to_string(&mut serialized_groups).unwrap();
        let groups: Vec<Group> = serde_json::from_str(&serialized_groups).expect("Could not create graph from file!");
        groups
    }

    /// converts the list of list of edges to a list of lists of strings
    pub fn paths_to_string(&self) -> Vec<Vec<String>> {
        let mut paths = Vec::with_capacity(self.paths.len());

        for path in self.paths.iter() {
            paths.push(path.edges.iter().map(|edge| edge.index().to_string()).collect())
        }

        paths
    }
}
