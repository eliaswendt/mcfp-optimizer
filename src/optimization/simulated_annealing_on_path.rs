use std::{
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};

use colored::Colorize;
use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{
    graph_weight::{TimetableEdge, TimetableNode},
    group::Group,
};

/// uses simulated annealing to improve parts of paths
///
/// first selects a random overcrowded edge, second selects one of its occupying groups and 
/// third changes the last part of the selected path of the group to detour the overcrowded edge
pub fn simulated_annealing<'a>(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &'a mut Vec<Group>,
    state: SelectionState<'a>,
    filepath: &str,
    n_iterations: u64
) -> SelectionState<'a> {
    println!("simulated_annealing_on_path()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(format!("{}.{}", filepath, "csv")).expect(&format!("Could not create file \"{}.csv\"", filepath)),
    );

    writer
        .write("time,temperature,cost,edge_cost,travel_cost,delay_cost\n".as_bytes())
        .unwrap();

    let mut r_writer = BufWriter::new(
        File::create(format!("{}_{}.{}", filepath, "runtime", "csv")).expect(&format!("Could not create file \"{}\"", format!("{}_{}.{}", filepath, "runtime", "csv"))),
    );

    r_writer
        .write("runtime,time\n".as_bytes())
        .unwrap();

    let mut current_state = state;
    let mut time: u64 = 1;

    let start_instant = Instant::now();

    loop {
        // get new temperature
        let temperature = n_iterations as f64 / time as f64;

        print!(
            "[time={}]: cost={}, edge_cost={}, travel_cost={}, delay_cost={}, temp={:.2}, ",
            time,
            current_state.cost,
            current_state.strained_edges_cost,
            current_state.travel_cost,
            current_state.travel_delay_cost,
            temperature
        );
        writer
            .write(
                format!(
                    "{},{},{},{},{},{}\n",
                    time,
                    temperature,
                    current_state.cost,
                    current_state.strained_edges_cost,
                    current_state.travel_cost,
                    current_state.travel_delay_cost
                )
                .as_bytes(),
            )
            .unwrap();

        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            print!("-> return");
            println!(" (done in {}s)", start_instant.elapsed().as_secs());

            r_writer
            .write(
                format!(
                    "{}s,{}\n",
                    start_instant.elapsed().as_secs(),
                    time
                )
                .as_bytes(),
            )
            .unwrap();
            
            return SelectionState {
                groups: groups,
                cost: current_state.cost,
                strained_edges_cost: current_state.strained_edges_cost,
                travel_cost: current_state.travel_cost,
                travel_delay_cost: current_state.travel_delay_cost,
                groups_path_index: current_state.groups_path_index,
            };
        }

        // get one random overcrowded edge and its occupying groups by index
        let (edge, group_indices) =
            current_state.get_random_overcrowded_edge_with_groups(graph, groups, &mut rng);

        // find a detour for a random group in previously found groups
        let (group_index, path) =
            current_state.find_detour_for_random_group(graph, groups, group_indices, edge, &mut rng);
    
        
        match path {
            // another path was found
            Some(path) => {
                let old_path_index = current_state.groups_path_index[group_index];

                // add path to paths of group
                groups[group_index].paths.insert(0, path.clone());


                // create new state
                let next =
                    current_state.group_neighbor_from_group_and_path(graph, groups, group_index, 0);

                // if next_state is better than current_state -> delta positive
                // if next_state is worse than current_state -> delta negative
                let delta_cost = current_state.cost as i64 - next.cost as i64;

                print!("delta_cost={:4.}, ", delta_cost);

                if delta_cost > 0 {
                    current_state = next.clone();
                    println!("{}", format!("-> replacing current state").green());
                } else {
                    let probability = (delta_cost as f64 / 50.0 / temperature as f64).exp();
                    let random = rng.gen_range(0.0..1.0);

                    print!("probability={:.2}, random={:.2} ", probability, random);

                    if random < probability {
                        println!("{}", format!("-> choosing worse neighbor").red());
                        current_state = next.clone();
                    } else {
                        println!("-> skipping");
                        current_state.groups_path_index[group_index] = old_path_index + 1;
                    }
                }
            },
            // No other path was found
            None => {
                println!("-> skipping")
            }
        }

        // print!("next_state={:?}, ", next_state.groups_paths_selection);

        time += 1;
    }
}
