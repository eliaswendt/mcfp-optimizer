use std::{collections::{HashSet, HashMap}, iter::{from_fn}, time::Instant};

pub mod group;
pub mod footpath;
pub mod station;
pub mod trip;
pub mod algo;
mod path_finder;

use group::Group;

use petgraph::{EdgeDirection::{Outgoing}, Graph, dot::{Dot}, graph::{NodeIndex, EdgeIndex, DiGraph}};
use colored::*;


use crate::csv_reader;
use indexmap::IndexSet;

/// Node Type of the DiGraph
#[derive(Debug, Clone)]
pub enum NodeWeight {
    Departure { // departure of a train ride
        trip_id: u64,
        time: u64,
        station_id: String,
        station_name: String,
    },

    Arrival { // arrival of a train ride
        trip_id: u64,
        time: u64,
        station_id: String,
        station_name: String,
    },

    Transfer { // transfer node at a station, existing for every departure at that station
        time: u64,
        station_id: String,
        station_name: String,
    },

    MainArrival {
        station_id: String,
    }
}

impl NodeWeight {

    pub fn get_time(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id: _, time, station_id: _, station_name: _} => Some(*time),
            Self::Arrival {trip_id: _, time, station_id: _, station_name: _} => Some(*time),
            Self::Transfer {time, station_id: _, station_name: _} => Some(*time),
            _ => None
        }
    }

    pub fn get_station_id(&self) -> Option<String> {
        match self {
            Self::Departure {trip_id: _, time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::Transfer {time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::MainArrival {station_id} => Some(station_id.clone()),
            _ => None
        }
    }

    pub fn is_arrival_at_station(&self, target_station_id: &str) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => station_id == target_station_id,
            _ => false
        }
    }

    pub fn is_departure(&self) -> bool {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    pub fn is_arrival(&self) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    pub fn is_transfer(&self) -> bool {
        match self {
            Self::Transfer {time: _, station_id: _, station_name: _}  => true,
            _ => false
        }
    }

    pub fn is_main_arrival(&self) -> bool {
        match self {
            Self::MainArrival {station_id: _} => true,
            _ => false
        }
    }

    pub fn get_kind(&self) -> &str {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => "Departure",
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => "Arrival",
            Self::Transfer {time: _, station_id: _, station_name: _}  => "Transfer",
            Self::MainArrival {station_id: _} => "MainArrival",
        }
    }

    pub fn get_trip_id(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Arrival {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Transfer {time: _, station_id: _, station_name: _}  => None,
            Self::MainArrival {station_id: _} => None,
        }
    }
}

/// Edge Type of the DiGraph
#[derive(Debug, Clone)]
pub enum EdgeWeight {
    Ride { // edge between departure and arrival
        duration: u64,
        capacity: u64,
        utilization: u64,
    },

    WaitInTrain { // edge between arrival and departure in the same train (stay in the train)
        duration: u64
    },
    
    Board, // edge between transfer node and departure

    Alight { // edge between arrival and transfer
        duration: u64
    },

    WaitAtStation { // edge between two transfer nodes
        duration: u64
    },

    Walk { // edge between arrival and next transfer node at other station
        duration: u64
    },

    MainArrivalRelation // connects all arrivals to MainArrival node
}


impl EdgeWeight {

    // maps every edge to some virtual cost for improved DFS (aka. effort/expense to "take" the edge)
    pub fn cost(&self) -> u64 {
        match self {
            Self::Ride {duration: _, capacity: _, utilization: _} => 2,
            Self::WaitInTrain {duration: _} => 1,
            Self::Alight {duration: _} => 4,
            Self::WaitAtStation {duration: _} => 3,
            Self::Walk {duration: _} => 10,
            Self::Board => 5,
            Self::MainArrivalRelation => 0 // no cost, just a "meta" path
        }
    }


    /// is RideToStation Edge
    pub fn is_ride(&self) -> bool {
        match self {
            Self::Ride {
                duration: _, 
                capacity: _, 
                utilization: _
            } => true,
            _ => false,
        }
    }

    /// is WaitInTrain Edge
    pub fn is_wait_in_train(&self) -> bool {
        match self {
            Self::WaitInTrain {
                duration: _, 
            } => true,
            _ => false,
        }
    }

    /// is Footpath Edge
    pub fn is_walk(&self) -> bool {
        match self {
            Self::Walk {
                duration: __
            } => true,
            _ => false,
        }
    }

    pub fn is_alight(&self) -> bool {
        match self {
            Self::Alight {
                duration: _
            } => true,
            _ => false
        }
    }

    pub fn is_wait_at_station(&self) -> bool {
        match self {
            Self::WaitAtStation {
                duration: _
            } => true,
            _ => false
        }
    }

    pub fn is_board(&self) -> bool {
        match self {
            Self::Board => true,
            _ => false
        }
    }
    
    pub fn is_main_arrival_relation(&self) -> bool {
        match self {
            Self::MainArrivalRelation => true,
            _ => false
        }
    }

    /// get duration of self, defaults to 0
    pub fn get_duration(&self) -> u64 {
        match self {
            Self::Ride{duration, capacity: _, utilization: _} => *duration,
            Self::WaitInTrain{duration} => *duration,
            Self::Alight{duration} => *duration,
            Self::WaitAtStation{duration} => *duration,
            Self::Walk{duration} => *duration,
            _ => 0,
        }
    }

    /// get capacity of self, defaults to MAX
    pub fn get_capacity(&self) -> u64 {
        match self {
            Self::Ride{duration: _, capacity, utilization: _} => *capacity,
            _ => std::u64::MAX, // all other edges are not limited in terms of capacity
        }
    }

    /// increase utilization of this edge by <addend>
    pub fn increase_utilization(&mut self, addend: u64) {
        match self {
            Self::Ride{duration: _, capacity: _, utilization} => *utilization += addend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    /// get utilization of self, defaults to 0
    pub fn get_utilization(&self) -> u64 {
        match self {
            Self::Ride{duration: _, capacity: _, utilization} => *utilization,
            _ => 0 // other edges always return 0 utilization as they have unlimited capacity
        }
    }

    pub fn get_remaining_capacity(&self) -> u64 {
        match self {
            Self::Ride{duration: _, capacity, utilization} => *capacity - *utilization,
            _ => u64::MAX // other edges always return u64::MAX as they have unlimited capacity
        }
    }

    pub fn get_kind_as_str(&self) -> &str {
        match self {
            Self::Ride {duration: _, capacity: _, utilization: _}  => "Ride",
            Self::WaitInTrain {duration: _} => "WaitInTrain",
            Self::Board => "Board",
            Self::Alight {duration: _} => "Alight",
            Self::WaitAtStation {duration: _} => "WaitAtStation",
            Self::Walk {duration: _} => "Walk",
            Self::MainArrivalRelation => "MainArrivalRelation"
        }
    }
}

pub enum Object {
    Edge(EdgeWeight),
    Node(NodeWeight)
}

pub enum ObjectIndex {
    EdgeIndex(EdgeIndex),
    NodeIndex(NodeIndex),
}

/// entire combined data model
pub struct Model {
    pub graph: DiGraph<NodeWeight, EdgeWeight>,

    // we need to store all transfer and arrival nodes for all stations at all times
    stations_transfers: HashMap<String, Vec<(u64, NodeIndex)>>,
    stations_main_arrival: HashMap<String, NodeIndex>
}

impl Model {

    pub fn with_stations_footpaths_and_trips(csv_folder_path: &str) -> Self {

        let start = Instant::now();

        let station_maps = csv_reader::read_to_maps(&format!("{}stations.csv", csv_folder_path));
        let trip_maps = csv_reader::read_to_maps(&format!("{}trips.csv", csv_folder_path));
        let footpath_maps = csv_reader::read_to_maps(&format!("{}footpaths.csv", csv_folder_path));

        // create graph
        let mut graph = DiGraph::new();

        let mut stations = station::Station::from_maps_to_map(&station_maps, &mut graph);
        let mut stations_transfers = HashMap::with_capacity(stations.len());
        let mut stations_arrivals = HashMap::with_capacity(stations.len());
        let mut stations_main_arrival = HashMap::with_capacity(stations.len());

        let trips = trip::Trip::from_maps_to_vec(&trip_maps);
        let footpaths = footpath::Footpath::from_maps_to_vec(&footpath_maps);

        for trip in trips.iter() {

            let from_station = stations.get_mut(&trip.from_station).unwrap();
            let departure = from_station.add_departure(&mut graph, trip.id, trip.departure);

            let to_station = stations.get_mut(&trip.to_station).unwrap();
            let arrival = to_station.add_arrival(&mut graph, trip.id, trip.arrival);

            // connect start and end of this ride
            graph.add_edge(departure, arrival, EdgeWeight::Ride {
                capacity: trip.capacity,
                duration: trip.arrival - trip.departure,
                utilization: 0
            });
        }

        for (station_id, station) in stations.into_iter() {
            let (transfers, arrivals) = station.connect(&mut graph);

            // create main arrival node
            let main_arrival = graph.add_node(NodeWeight::MainArrival {
                station_id: station_id.clone()            
            });

            // connect all arrival nodes to the main arrival
            for arrival in arrivals.iter() {
                // connect arrival to station's main node
                graph.add_edge(*arrival, main_arrival, EdgeWeight::MainArrivalRelation);
            }

            // save references to all transfers and to arrival_main
            stations_transfers.insert(station_id.clone(), transfers);
            stations_arrivals.insert(station_id.clone(), arrivals);
            stations_main_arrival.insert(station_id.clone(),main_arrival);
        }


        let mut successful_footpath_counter: u64 = 0;
        let mut failed_footpath_counter: u64 = 0;

        // iterate over all footpaths
        for footpath in footpaths.iter() {
            let from_station_arrivals = stations_arrivals.get(&footpath.from_station).unwrap();
            let to_station_transfers = stations_transfers.get(&footpath.to_station).unwrap();

            // for every arrival at the from_station try to find the next transfer node at the to_station
            for arrival in from_station_arrivals.iter() {
                let arrival_time = graph[*arrival].get_time().unwrap();

                // timestamp of arrival at the footpaths to_station
                let earliest_transfer_time = arrival_time + footpath.duration;

                let mut edge_added = false;

                // try to find next transfer node at to_station (requires transfers to be sorted, earliest first)
                for (transfer_time, transfer) in to_station_transfers.iter() {

                    if earliest_transfer_time <= *transfer_time {
                        graph.add_edge(*arrival, *transfer, EdgeWeight::Walk {
                            duration: footpath.duration
                        });
                        edge_added = true;
                        successful_footpath_counter += 1;
                        break // the inner loop
                    }
                }

                if !edge_added {
                    failed_footpath_counter += 1;
                    //println!("There couldn't be found any valid (time) transfer node for footpath from {} -> {}", footpath.from_station, footpath.to_station);
                }
            }
        }

        println!("successful_footpaths: {}, failed_footpaths: {}", successful_footpath_counter, failed_footpath_counter);

        println!(
            "[with_stations_footpaths_and_trips()]: done ({}ms), graph.node_count()={}, graph.edge_count()={}", 
            start.elapsed().as_millis(),
            graph.node_count(), 
            graph.edge_count()
        );

        Self {
            graph,
            stations_transfers,
            stations_main_arrival
        }
    }

    pub fn to_dot(&self) -> String {
        format!("{:?}", Dot::with_config(&self.graph, &[]))
    }


    pub fn find_solutions(&mut self, groups_csv_filepath: &str) {
        // Bei den Reisendengruppen gibt es noch eine Änderung: Eine zusätzliche Spalte "in_trip" gibt jetzt an, in welchem Trip sich die Gruppe aktuell befindet. Die Spalte kann entweder leer sein (dann befindet sich die Gruppe aktuell in keinem Trip, sondern an der angegebenen Station) oder eine Trip ID angeben (dann befindet sich die Gruppe aktuell in diesem Trip und kann frühestens an der angegebenen Station aussteigen).
        // Das beeinflusst den Quellknoten der Gruppe beim MCFP: Befindet sich die Gruppe in einem Trip sollte der Quellknoten der entsprechende Ankunftsknoten (oder ein zusätzlich eingefügter Hilfsknoten, der mit diesem verbunden ist) sein. Befindet sich die Gruppe an einer Station, sollte der Quellknoten ein Warteknoten an der Station (oder ein zusätzlich eingefügter Hilfsknoten, der mit diesem verbunden ist) sein.
        // Falls die Gruppe an einer Station startet, muss in diesem Fall am Anfang die Stationsumstiegszeit berücksichtigt werden (kann man sich so vorstellen: die Gruppe steht irgendwo an der Station und muss erst zu dem richtigen Gleis laufen).
        // Befindet sich die Gruppe hingegen in einem Trip, hat sie zusätzlich die Möglichkeit, mit diesem weiterzufahren und erst später umzusteigen. (Würde man sie an der Station starten lassen, wäre die Stationsumstiegszeit nötig, um wieder in den Trip einzusteigen, in dem sie eigentlich schon ist - und meistens ist die Standzeit des Trips geringer als die Stationsumstiegszeit)
        // Habe auch die Formatbeschreibung im handcrafted-scenarios Repo entsprechend angepasst.


        let group_maps = csv_reader::read_to_maps(groups_csv_filepath);
        let groups_map = Group::from_maps_to_map(&group_maps);

        // create vector of groups and sort them (highest passenger count first)
        let mut groups_sorted: Vec<&Group> = groups_map.values().collect();
        groups_sorted.sort_unstable_by_key(|group| group.passengers);
        groups_sorted.reverse();

        for group_value in groups_sorted.into_iter(){

            let from_node_index = self.find_start_node_index(&group_value.start, group_value.departure).expect("Could not find departure at from_station");
            let to_node_index = self.find_end_node_index(&group_value.destination).expect("Could not find destination station");

            // max duration should depend on the original travel time
            let travel_time = group_value.arrival - group_value.departure;
            let max_duration = (travel_time as f64 * 2.0) as u64; // todo: factor to modify later if not a path could be found for all groups

            let start = Instant::now();
            print!("[group={}]: {} -> {} with {} passenger(s) in {} min(s) ... ", group_value.id, group_value.start, group_value.destination, group_value.passengers, max_duration);

            let mut paths_recursive = path_finder::all_paths_dfs_recursive(
                &self.graph, 
                from_node_index, 
                to_node_index, //|node| node.is_arrival_at_station(&group_value.destination), // dynamic condition for dfs algorithm to find arrival node

                group_value.passengers as u64, 
                max_duration, 
                100 // initial budget for cost (each edge has individual search cost)
            );
            
            //let mut paths_recursive = Self::all_simple_paths_dfs_dorian(&self.graph, from_node_index, to_node_index, max_duration, 25).collect::<Vec<_>>();

            print!("done in {}ms ... ", start.elapsed().as_millis());

            // sort by remaining time and number of edges
            // sort paths by remaining duration (highest first)
            // paths_recursive.sort_by(|(x_remaining_duration, x_path), (y_remaining_duration, y_path)| {
            //     match x_remaining_duration.cmp(y_remaining_duration).reverse() {
            //         std::cmp::Ordering::Equal => x_path.len().cmp(&y_path.len()), // prefer less edges -> should be less transfers
            //         other => other, 
            //     }
            // });
            paths_recursive.sort_unstable_by_key(|(remaining_duration, _)| *remaining_duration);
            paths_recursive.reverse();

            let output = match paths_recursive.first() {
                Some((remaining_duration, path)) => {

                    for edge_index in path.iter() {
                        
                        self.graph.edge_weight_mut(*edge_index).unwrap().increase_utilization(group_value.passengers as u64);
                    }

                    format!("augmenting best path (remaining_duration={}, len={}, total_number_paths={})", remaining_duration, path.len(), paths_recursive.len()).green()
                },

                None => {
                    "no path to augment".red()
                }
            };

            println!("{}", output);
            
            //let paths_recursive = self.all_simple_paths_dfs_dorian(from_node_index, to_node_index, max_duration, 5);

            // let mut only_paths = Vec::new();
            // for (_, path) in paths_recursive {
            //     only_paths.push(path.clone())
            // }

            

            // let all_edges_in_paths_recursive: HashSet<EdgeIndex> = only_paths.iter().flatten().cloned().collect();
            // if all_edges_in_paths_recursive.len() > 0 {
            //     let subgraph = self.build_subgraph_with_edges(&all_edges_in_paths_recursive);
            //     println!("node_count_subgraph={}, edge_count_subgraph={}", subgraph.node_count(), subgraph.edge_count());

            //     BufWriter::new(File::create(format!("graphs/groups/{}.dot", group_value.id)).unwrap()).write(
            //         format!("{:?}", Dot::with_config(&subgraph, &[])).as_bytes()
            //     ).unwrap();
            // }

            // let subgraph_paths = self.create_subgraph_with_nodes(&mut subgraph, paths_recursive, &mut node_index_graph_subgraph_mapping);
    
            // let dot_code = format!("{:?}", Dot::with_config(&subgraph, &[]));
    
            // BufWriter::new(File::create(format!("graphs/subgraph_group_{}.dot", group_key)).unwrap()).write(
            //     dot_code.as_bytes()
            // ).unwrap();
        }

        // let dot_code = format!("{:?}", Dot::with_config(&subgraph, &[]));
    
        // BufWriter::new(File::create(format!("graphs/subgraph_complete.dot")).unwrap()).write(
        //     dot_code.as_bytes()
        // ).unwrap();


        // todo: iterate groups, augment routes ... return solutions
    }


    pub fn find_start_node_index(&self, station_id: &str, start_time: u64) -> Option<NodeIndex> {
        match self.stations_transfers.get(station_id) {
            Some(station_departures) => {
                
                // iterate until we find a departure time >= the time we want to start
                for (departure_time, departure_node_index) in station_departures.iter() {
                    if start_time <= *departure_time {
                        return Some(*departure_node_index);
                    }
                }

                // no departure >= start_time found
                None
            },

            // station not found
            None => None
        }
    }


    pub fn find_end_node_index(&self, station_id: &str) -> Option<NodeIndex> {
        self.stations_main_arrival.get(station_id).map(|main_arrival| *main_arrival)
    }


    /// builds subgraph that only contains nodes connected by edges
    pub fn build_subgraph_with_edges(&self, edges: &HashSet<EdgeIndex>) -> DiGraph<NodeWeight, EdgeWeight> {

        self.graph.filter_map(
            |node_index, node_weight| {

                // check if at least one incoming/outgoing edge of current node is in HashSet of edges
                let mut walker = self.graph.neighbors_undirected(node_index).detach();
                while let Some((current_edge, _)) = walker.next(&self.graph) {
                    if edges.contains(&current_edge) {
                        return Some(node_weight.clone());
                    }
                }

                // no edge in set -> do not include node in graph
                None
            },
            |edge_index, edge_weight| {
                if edges.contains(&edge_index) {
                    Some(edge_weight.clone())
                } else {
                    None
                }
            }
        )
    }


    // creates a subgraph of self with only the part of the graph of specified paths
    pub fn create_subgraph_with_nodes(&self, subgraph: &mut Graph<NodeWeight, EdgeWeight>, paths: Vec<Vec<NodeIndex>>, node_index_graph_subgraph_mapping: &mut HashMap<NodeIndex, NodeIndex>) -> Vec<Vec<ObjectIndex>> {
        //let mut subgraph = DiGraph::new();
        let mut subgraph_paths: Vec<Vec<ObjectIndex>> = Vec::new();

        // iterate all paths in graph
        for path in paths {

            let mut subgraph_path_indices: Vec<ObjectIndex> = Vec::new();
            let mut path_max_flow: u64 = std::u64::MAX;
            let mut path_edge_indices: Vec<EdgeIndex> = Vec::new();

            // iterate over all NodeIndex pairs in this path
            for graph_node_index_pair in path.windows(2) {

                // check if the first node already exists in subgraph
                let subgraph_node_a_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[0]) {
                    Some(subgraph_node_index) => *subgraph_node_index,
                    None => {
                        // clone NodeWeight from graph
                        let node_weight = self.graph.node_weight(graph_node_index_pair[0]).unwrap().clone();

                        // create new node in subgraph
                        let subgraph_node_index = subgraph.add_node(node_weight);
                        
                        // insert mapping into HashMap
                        node_index_graph_subgraph_mapping.insert(graph_node_index_pair[0], subgraph_node_index.clone());

                        subgraph_node_index
                    }
                };
                    
                // check if the second node already exists in subgraph
                let subgraph_node_b_index = match node_index_graph_subgraph_mapping.get(&graph_node_index_pair[1]) {
                    Some(subgraph_node_index) => *subgraph_node_index,
                    None => {
                        // clone NodeWeight from graph
                        let node_weight = self.graph.node_weight(graph_node_index_pair[1]).unwrap().clone();

                        // create new node in subgraph
                        let subgraph_node_index = subgraph.add_node(node_weight);
                        
                        // insert mapping into HashMap
                        node_index_graph_subgraph_mapping.insert(graph_node_index_pair[1], subgraph_node_index);

                        subgraph_node_index
                    }
                };
                
                // add outgoing node to path if path is empty
                if subgraph_path_indices.is_empty() {
                    subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_a_index));
                };

                // create edge if there was created at least one new node
                let subgraph_edge_weight = match subgraph.find_edge(subgraph_node_a_index, subgraph_node_b_index) {
                    Some(subgraph_edge_index) => {
                        // add edge to path
                        subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
                        path_edge_indices.push(subgraph_edge_index);
                        subgraph.edge_weight(subgraph_edge_index).unwrap()
                    },
                    None => {
                        let graph_edge_index = self.graph.find_edge(graph_node_index_pair[0], graph_node_index_pair[1]).unwrap();
                        let subgraph_edge_weight = self.graph.edge_weight(graph_edge_index).unwrap().clone();

                        let subgraph_edge_index = subgraph.add_edge(subgraph_node_a_index, subgraph_node_b_index, subgraph_edge_weight);
                        // add edge to path
                        subgraph_path_indices.push(ObjectIndex::EdgeIndex(subgraph_edge_index));
                        path_edge_indices.push(subgraph_edge_index);
                        subgraph.edge_weight(subgraph_edge_index).unwrap()
                    }
                };

                // update max_flow if edge capacity is smaller current path_max_flow
                let edge_remaining_flow = subgraph_edge_weight.get_capacity() - subgraph_edge_weight.get_utilization();
                if edge_remaining_flow < path_max_flow {
                    path_max_flow = edge_remaining_flow;
                };
                
                subgraph_path_indices.push(ObjectIndex::NodeIndex(subgraph_node_b_index));
            };

            subgraph_paths.push(subgraph_path_indices);

            // set utilization to all edges of path
            for path_edge_index in path_edge_indices {
                subgraph.edge_weight_mut(path_edge_index).unwrap().increase_utilization(path_max_flow);
                //println!("{}, {}", path_max_flow, subgraph.edge_weight(path_edge_index).unwrap().get_utilization())
            }
        }

        subgraph_paths
    }

    /// Panics if invalid
    pub fn validate_graph_integrity(&self) {

        let start = Instant::now();

        for node_a_index in self.graph.node_indices() {
            let node_a_weight = self.graph.node_weight(node_a_index).unwrap();
            
            let mut children = self.graph.neighbors_directed(node_a_index, Outgoing).detach();

            // Number of MainArrival node found for Arrival node
            let mut num_main_arrival = 0;
            // Number of WaitInTrain edges for Arrival node
            let mut num_wait_in_train = 0;
            // Number of Board edges for Transfer node
            let mut num_board = 0;

            while let Some((edge_index, child_b_index)) = children.next(&self.graph){
                // Check valid successor
                let edge_weight = self.graph.edge_weight(edge_index).unwrap();
                let node_b_weight = self.graph.node_weight(child_b_index).unwrap();


                // check node relation
                match node_a_weight {
                    NodeWeight::Departure {trip_id: _, time: _, station_id: _, station_name: _} => {

                        // Departure outgoing edge is ride
                        let edge_is_ride = edge_weight.is_ride();
                        assert!(edge_is_ride, format!("Outgoing edge of departure node is not Ride but {}!", edge_weight.get_kind_as_str()));
                        
                        // Outgoing Edge ends in Arrival node
                        let departure_to_arrival =  node_b_weight.is_arrival();
                        assert!(departure_to_arrival, format!("Node Departure does not end in Arrival node but in {}!", node_b_weight.get_kind()));
                        
                        // Departure time is before Arrival time
                        let departure_before_arrival = node_a_weight.get_time().unwrap() <= node_b_weight.get_time().unwrap();
                        assert!(departure_before_arrival, format!("Node Departure has greater time as Arrival node! {} vs {}", node_a_weight.get_time().unwrap(), node_b_weight.get_time().unwrap()));
                        
                        // Departure node has only one outgoing edge
                        let one_outgoing = self.graph.neighbors(node_a_index).enumerate().count();
                        assert!(one_outgoing == 1, format!("Departure node has not one outgoing edge but {}", one_outgoing));
                    
                        // both nodes have same trip 
                        let same_trip = node_a_weight.get_trip_id().unwrap() == node_b_weight.get_trip_id().unwrap();
                        assert!(same_trip == true, format!("Departure node has not the same trip as Arrival node! {} vs {}", node_a_weight.get_trip_id().unwrap(), node_b_weight.get_trip_id().unwrap()));
                    },

                    NodeWeight::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => {

                        // Outgoing edge is WaitInTrain, Alight, Walk, or MainArrivalRelation
                        let edge_is_correct = edge_weight.is_wait_in_train() || edge_weight.is_alight()
                            || edge_weight.is_walk() || edge_weight.is_main_arrival_relation();
                        assert!(edge_is_correct, format!("Outgoing edge of arrival node is not WaitInStation, Alight, Walk, or MainArrivalRelation but {}!", edge_weight.get_kind_as_str()));
                        
                        // if edge is WaitInTrain -> Nodes have same trip and node b is departure
                        if edge_weight.is_wait_in_train() {
                            let arrival_to_departure = node_b_weight.is_departure();
                            assert!(arrival_to_departure, format!("Node Arrival does not end in Departure node after WaitInTrain edge but in {}!", node_b_weight.get_kind()));
                            
                            num_wait_in_train += 1;

                            // same trip id
                            let same_trip = node_a_weight.get_trip_id().unwrap() == node_b_weight.get_trip_id().unwrap();
                            assert!(same_trip == true, format!("Arrival node has not the same trip as Departure node for WaitInStation edge! {} vs {}", node_a_weight.get_trip_id().unwrap(), node_b_weight.get_trip_id().unwrap()));
                        }

                        // if edge is Alight -> node b is transfer
                        if edge_weight.is_alight() {
                            let arrival_to_transfer = node_b_weight.is_transfer();
                            assert!(arrival_to_transfer, format!("Node Arrival does not end in Transfer node after Alight edge but in {}!", node_b_weight.get_kind()));
                        }

                        // if edge is Walk -> node b is transfer
                        if edge_weight.is_walk() {
                            let arrival_to_walk = node_b_weight.is_transfer();
                            assert!(arrival_to_walk, format!("Node Arrival does not end in Transfer node after Walk edge but in {}!", node_b_weight.get_kind()));
                        }

                        // if edge is MainStationRelation -> node b is MainArrival
                        if edge_weight.is_main_arrival_relation() {
                            let arrival_to_main_arrival = node_b_weight.is_main_arrival();
                            assert!(arrival_to_main_arrival, format!("Node Arrival does not end in Transfer node after MainArrivalRelation edge but in {}!", node_b_weight.get_kind()));
                            num_main_arrival += 1;
                        }

                        // Arrival node has time before node b
                        if node_b_weight.is_departure() || node_b_weight.is_transfer() {
                            let arrival_before_departure_transfer = node_a_weight.get_time().unwrap() <= node_b_weight.get_time().unwrap();
                            assert!(arrival_before_departure_transfer, format!("Node Arrival has greater time as {} node! {} vs {}", node_b_weight.get_kind(), node_a_weight.get_time().unwrap(), node_b_weight.get_time().unwrap()));
                        }

                        // Arrival node and node b have same stations
                        if node_b_weight.is_departure() || node_b_weight.is_main_arrival() {
                            // same stations
                            let same_stations = node_a_weight.get_station_id().unwrap() == node_b_weight.get_station_id().unwrap();
                            assert!(same_stations, format!("Arrival node and {} node have not same station! {} vs. {}", node_b_weight.get_kind(), node_a_weight.get_station_id().unwrap(), node_b_weight.get_station_id().unwrap()));
                        }
                    },
                    NodeWeight::Transfer {time: _, station_id: _, station_name: _} => {

                        // Outgoing edge is Board or WaitAtStation
                        let edge_is_correct = edge_weight.is_board() || edge_weight.is_wait_at_station();
                        assert!(edge_is_correct, format!("Outgoing edge of Transfer node is not Board, or WaitAtStation but {}!", edge_weight.get_kind_as_str()));
                        
                        // if edge is Board -> node b is Departure node and both have same time
                        if edge_weight.is_board() {
                            let transfer_to_departure = node_b_weight.is_departure();
                            assert!(transfer_to_departure, format!("Node Transfer does not end in Departure node after Board edge but in {}!", node_b_weight.get_kind()));

                            let same_time = node_a_weight.get_time().unwrap() == node_b_weight.get_time().unwrap();
                            assert!(same_time, format!("Transfer node and Departure node have not same time! {} vs. {}", node_a_weight.get_time().unwrap(), node_b_weight.get_time().unwrap()));
                        
                            num_board += 1;
                        }

                        // if edge is WaitAtStation -> node b is Transfer node and node b has time greater or equal node a
                        if edge_weight.is_wait_at_station() {
                            let transfer_to_transfer = node_b_weight.is_transfer();
                            assert!(transfer_to_transfer, format!("Node Transfer does not end in Transfer node after WaitAtStation edge but in {}!", node_b_weight.get_kind()));
                        
                            let transfer_before_transfer = node_a_weight.get_time().unwrap() <= node_b_weight.get_time().unwrap();
                            assert!(transfer_before_transfer, format!("Transfer node has not time less or equal Transfer node! {} vs. {}", node_a_weight.get_time().unwrap(), node_b_weight.get_time().unwrap()));
                        }

                        // both nodes have same station
                        let same_stations = node_a_weight.get_station_id().unwrap() == node_b_weight.get_station_id().unwrap();
                        assert!(same_stations, format!("Transfer node and {} node have not same station! {} vs. {}", node_b_weight.get_kind(), node_a_weight.get_station_id().unwrap(), node_b_weight.get_station_id().unwrap()));                   
                    },
                    NodeWeight::MainArrival {station_id: _} => {
                        
                        // todo: Panic because MainArrival node has no outgoing edges
                        assert!(false, "MainArrival node has outgoing edge!")
                    }
                }
            }

            // check node on its own
            match node_a_weight {
                NodeWeight::Departure {trip_id: _, time: _, station_id: _, station_name: _} => {
                    
                    // Exactly one outgoing edge
                    let num_edges = self.graph.edges_directed(node_a_index, Outgoing).count();
                    assert!(num_edges == 1, format!("Departure node has {} outgoing edges instead of one!", num_edges));
                },
                NodeWeight::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => {
                    
                    // Only one MainArrival node found
                    if num_main_arrival != 1 {
                        println!("Outgoing edges:");
                        let mut children = self.graph.neighbors_directed(node_a_index, Outgoing).detach();
                        while let Some((child_edge_index, child_node_index)) = children.next(&self.graph) {
                            println!("{:?}", self.graph.edge_weight(child_edge_index).unwrap());
                        }
                    }
                    assert!(num_main_arrival == 1, format!("Arrival node has {} MainArrival nodes instead of 1!", num_main_arrival));

                    // Max one WaitInTrain outgoing edge per Arrival
                    assert!(num_wait_in_train <= 1, format!("Arrival node has {} outgoing WaitInTrain edges instead of 0 or 1!", num_wait_in_train));
                },
                NodeWeight::Transfer {time: _, station_id: _, station_name: _} => {

                    // Only one outoging board edge
                    assert!(num_board == 1, format!("Transfer node has {} outgoing Board edges instead of 1!", num_board));
                },
                NodeWeight::MainArrival {station_id: _} => {
                        
                    // has no outgoing edges
                    let outoging_edge_count = self.graph.edges_directed(node_a_index, Outgoing).count();
                    assert!(outoging_edge_count == 0, format!("MainArrival node has {} outgoing Board edges instead of 0!", outoging_edge_count));
                }
            }
        }

        println!("[validate_graph_integrity()]: passed ({}ms)", start.elapsed().as_millis());
    }
}
