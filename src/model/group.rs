use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::HashMap,
    fmt,
    fs::File,
    io::{BufReader, BufWriter},
    time::Instant,
};

use colored::Colorize;

use super::{
    path::{self, Path},
    trip::Trip,
    Model,
};

/// travel group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,

    pub start_station_id: u64, // Start-Halt für die Alternativensuche (Station ID)
    pub destination_station_id: u64, // Ziel der Gruppe (Station ID)

    pub departure_time: u64, // Frühstmögliche Abfahrtszeit am Start-Halt (Integer)
    pub arrival_time: u64,   // Ursprünglich geplante Ankunftszeit am Ziel (Integer)

    pub passengers: u64, // Größe der Gruppe (Integer)

    // Hier gibt es zwei Möglichkeiten (siehe auch unten):
    // Wenn der Wert leer ist, befindet sich die Gruppe am Start-Halt.
    // Wenn der Wert nicht leer ist, gibt er die Trip ID (Integer) der Fahrt an, in der sich die Gruppe befindet.
    pub in_trip: Option<u64>,

    pub paths: Vec<Path>, // possible paths for this group
}

impl Group {
    pub fn from_maps_to_vec(group_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {
        println!("parsing {} group(s)", group_maps.len());

        let mut groups = Vec::with_capacity(group_maps.len());

        for group_map in group_maps.iter() {
            let id = group_map.get("id").unwrap().parse().unwrap();

            let in_trip_value = group_map.get("in_trip").unwrap();
            let in_trip: Option<u64> = if in_trip_value.is_empty() {
                None
            } else {
                Some(in_trip_value.parse().unwrap())
            };

            groups.push(Self {
                id,
                start_station_id: group_map.get("start").unwrap().parse().unwrap(),
                destination_station_id: group_map.get("destination").unwrap().parse().unwrap(),
                departure_time: group_map.get("departure").unwrap().parse().unwrap(),
                arrival_time: group_map.get("arrival").unwrap().parse().unwrap(),
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
            File::create(&format!("{}groups.bincode", filepath))
                .expect(&format!("Could not open file {}groups.bincode", filepath)),
        );
        bincode::serialize_into(writer, groups).expect("Could not save groups to file");
        // serde_json::to_writer(writer, groups).expect("Could not save groups to file");

        println!("done ({}ms)", start.elapsed().as_millis());
    }

    pub fn load_from_file(filepath: &str) -> Vec<Self> {
        print!("loading groups from {} ... ", filepath);
        let start = Instant::now();

        let reader = BufReader::new(
            File::open(&format!("{}groups.bincode", filepath))
                .expect(&format!("Could not open file {}model.bincode", filepath)),
        );
        let groups: Vec<Group> =
            bincode::deserialize_from(reader).expect("Could not load groups from file!");
        // let groups: Vec<Group> = serde_json::from_reader(reader).expect("Could not load groups from file!");

        println!("done ({}ms)", start.elapsed().as_millis());

        groups
    }

    /// returns the number of found paths
    pub fn search_paths(&mut self, model: &Model) {
        // TODO: Bei den Reisendengruppen gibt es noch eine Änderung: Eine zusätzliche Spalte "in_trip" gibt jetzt an, in welchem Trip sich die Gruppe aktuell befindet. Die Spalte kann entweder leer sein (dann befindet sich die Gruppe aktuell in keinem Trip, sondern an der angegebenen Station) oder eine Trip ID angeben (dann befindet sich die Gruppe aktuell in diesem Trip und kann frühestens an der angegebenen Station aussteigen).
        // Das beeinflusst den Quellknoten der Gruppe beim MCFP: Befindet sich die Gruppe in einem Trip sollte der Quellknoten der entsprechende Ankunftsknoten (oder ein zusätzlich eingefügter Hilfsknoten, der mit diesem verbunden ist) sein. Befindet sich die Gruppe an einer Station, sollte der Quellknoten ein Warteknoten an der Station (oder ein zusätzlich eingefügter Hilfsknoten, der mit diesem verbunden ist) sein.

        // find next start node at station with specified id from this start_time
        // returns the first timely reachable transfer at the station_id
        // returns None if no transfer reachable
        let start: NodeIndex = match self.in_trip {
            Some(in_trip) => {
                // in_trip is set -> start at arrival of current trip

                // println!("start={}, in_trip={}, departure={}", self.start, in_trip, self.departure);

                // FIRST: get all arrival nodes of the start station
                let start_station_arrivals =
                    model.stations_arrivals.get(&self.start_station_id).unwrap();

                // SECOND: search all arrivals for trip_id == in_trip AND time == start at start station
                let mut selected_station_arrival = None;
                for start_station_arrival in start_station_arrivals.iter() {
                    let arrival = &model.graph[*start_station_arrival];

                    if arrival.trip_id().unwrap() == in_trip
                        && arrival.time() == self.departure_time
                    {
                        selected_station_arrival = Some(*start_station_arrival);
                        // println!("Found arrival={:?}", arrival);
                        break;
                    }
                }

                selected_station_arrival.expect(&format!(
                    "Could not find arrival for in_trip={} and departure={}",
                    in_trip, self.departure_time
                ))
            }
            None => {
                // in_trip is not set -> start at station transfer

                let mut selected_station_transfer = None;

                match model.stations_transfers.get(&self.start_station_id) {
                    Some(station_transfers) => {
                        // iterate until we find a departure time >= the time we want to start
                        for station_transfer in station_transfers.iter() {
                            if self.departure_time <= model.graph[*station_transfer].time()
                            {
                                selected_station_transfer = Some(*station_transfer);
                                break;
                            }
                        }
                    }
                    None => {}
                }

                selected_station_transfer.expect("Could not find departure at from_station")
            }
        };

        let destination_station_name = model.graph
            [model.stations_arrivals.get(&self.destination_station_id).unwrap()[0]]
            .station_name();

        if self.departure_time > self.arrival_time {
            // invalid time

            println!(
                "{} -> {} ... arrival_time before departure_time -> skipping",
                model.graph[start].station_name(),
                destination_station_name
            );
            return;
        }

        // max duration should depend on the original travel time
        let travel_time = self.arrival_time - self.departure_time;

        //let max_duration = (travel_time as f64 * duration_factor) as u64; // todo: factor to modify later if not a path could be found for all groups

        let start_instant = Instant::now();
        print!(
            "{} -> {} .. ",
            model.graph[start].station_name(),
            destination_station_name,
        );

        // use iterative deepening search to find edge paths
        let edge_sets = path::Path::all_paths_iddfs(
            &model.graph,
            start,
            self.destination_station_id,
            100,

            2 * travel_time + 120,
            &vec![
                30, 35, 40, 45, 50, 55, 60
            ],
        );

        // let edge_sets = path::bfs(
        //     &model.graph,
        //     start,
        //     self.destination_station_id,

        //     1,
        //     u64::MAX,
        //     40
        // );


        // for (index, edge_set) in edge_sets.iter().enumerate() {

        //     println!();

        //     print!("[path_{}]: ", index);
        //     for edge in edge_set.iter() {
        //         print!("{:?} ", model.graph[*edge]);
        //     }

        //     println!("\nexpected start_node_station_id={:?}", model.graph[start].station_id());
        //     println!("expected destination_node_station_id={}", self.destination_station_id);

        //     println!("path_start={}, path_end={}",
        //         model.graph[model.graph.edge_endpoints(*edge_set.first().unwrap()).unwrap().0].station_id(),

        //         model.graph[model.graph.edge_endpoints(*edge_set.last().unwrap()).unwrap().1].station_id(),
        //     );
        // }

        // transform each edge_set into a full Path object
        self.paths = edge_sets
            .into_iter()
            .filter(|edge_set| edge_set.len() != 0) // filter out empty edge_sets (paths that don't have a single edge)
            .map(|edge_set| Path::new(&model.graph, edge_set, self.passengers, self.arrival_time))
            .collect();

        if self.paths.len() == 0 {

            self.paths = path::Path::dfs_visitor_search(
                &model.graph,
                start,
                self.destination_station_id,
                self.passengers as u64,
                self.arrival_time,
                0,
            );
        }

        // filter out paths that exceed duration or do not fulfill minium capacity

        print!("done in {}ms, ", start_instant.elapsed().as_millis());

        // sort lowest travel_cost first
        self.paths.sort_unstable();

        if self.paths.len() == 0 {
            println!("{}", format!("no path found").red());
        } else {
            println!(
                "{}",
                format!(
                    "{} path(s), best={{travel_cost={}, duration={}, len={}}}",
                    self.paths.len(),
                    self.paths[0].travel_cost(),
                    self.paths[0].duration(),
                    self.paths[0].edges.len()
                )
                .green()
            );
        }
    }

    fn calculate_max_travel_duration(travel_time: u64) -> u64 {
        (1.5 * travel_time as f64) as u64 + 200
    }
}
