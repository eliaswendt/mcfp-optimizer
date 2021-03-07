use serde::{Deserialize, Serialize};
use std::io::Read;
use std::io::Write;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    time::Instant,
};

use colored::Colorize;
use petgraph::graph::{DiGraph, EdgeIndex};

use super::{
    path::{self, Path},
    Model, TimetableEdge, TimetableNode,
};

/// travel group
#[derive(Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,

    pub start: String,       // Start-Halt für die Alternativensuche (Station ID)
    pub destination: String, // Ziel der Gruppe (Station ID)

    pub departure: u64, // Frühstmögliche Abfahrtszeit am Start-Halt (Integer)
    pub arrival: u64,   // Ursprünglich geplante Ankunftszeit am Ziel (Integer)

    pub passengers: usize, // Größe der Gruppe (Integer)

    // Hier gibt es zwei Möglichkeiten (siehe auch unten):
    // Wenn der Wert leer ist, befindet sich die Gruppe am Start-Halt.
    // Wenn der Wert nicht leer ist, gibt er die Trip ID (Integer) der Fahrt an, in der sich die Gruppe befindet.
    pub in_trip: Option<usize>,

    pub paths: Vec<Path>, // possible paths for this group
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
                paths: Vec::new(),
            });
        }

        groups
    }


    pub fn save_to_file(groups: &Vec<Group>, filepath: &str) {
        print!("saving groups to {} ... ", filepath);
        let start = Instant::now();

        let writer = BufWriter::new(
            File::create(&format!("{}groups.json", filepath))
                .expect(&format!("Could not open file {}groups.json", filepath)),
        );
        serde_json::to_writer(writer, groups).expect("Could not save groups to file");

        println!("done ({}ms)", start.elapsed().as_millis());
    }


    pub fn load_from_file(filepath: &str) -> Vec<Self> {
        print!("loading groups from {} ... ", filepath);
        let start = Instant::now();

        let reader = BufReader::new(
            File::open(&format!("{}groups.json", filepath))
                .expect(&format!("Could not open file {}model.json", filepath)),
        );
        let groups: Vec<Group> =
            serde_json::from_reader(reader).expect("Could not load groups from file!");

        println!("done ({}ms)", start.elapsed().as_millis());

        groups
    }










    /// returns (remaining_duration, path), returns true if there was at least one path found
    pub fn search_paths(&mut self, model: &Model, max_budget: u64, duration_factor: f64) -> bool {
        let from = model
            .find_start_node_index(&self.start, self.departure)
            .expect("Could not find departure at from_station");
        let to = model
            .find_end_node_index(&self.destination)
            .expect("Could not find destination station");

        // max duration should depend on the original travel time
        let travel_time = self.arrival - self.departure;
        let max_duration = (travel_time as f64 * duration_factor) as u64; // todo: factor to modify later if not a path could be found for all groups

        let start = Instant::now();
        print!(
            "{} -> {} with {} passenger(s) in {} min(s) ... ",
            self.start, self.destination, self.passengers, max_duration
        );
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
            1,
            100,
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
            println!(
                "{}",
                format!(
                    "{} path(s), best={{duration={}, len={}}}",
                    self.paths.len(),
                    self.paths[0].duration(),
                    self.paths[0].len()
                )
                .green()
            );
            true
        }
    }
}