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
    path::Path,
};

/// maps time to temperature value
fn time_to_temperature(time: f64) -> f64 {
    //(5000.0 / time).powf(1.2)
    2000.0 / time // cost=782, funktioniert schonmal ganz gut
                  // 10000.0 - time // funktioniert kaum, trend stimmt aber
}

pub fn simulated_annealing<'a>(
    graph: &'a mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &mut Vec<Group>,
    state: SelectionState,
    filepath: &str,
) {
    println!("simulated_annealing()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath)),
    );

    writer.write("time,temperature,cost\n".as_bytes()).unwrap();

    let new_group_list = groups.clone();
    let mut current_state = SelectionState::generate_random_state(graph, &new_group_list);
    //let mut current_state = state;
    let mut time = 1;

    let start_instant = Instant::now();

    // For termination condition
    let mut steps_without_changes = 0;

    loop {
        if steps_without_changes > 200 || current_state.cost <= 0 {
            print!("-> return with costs={} ", current_state.cost);
            println!("(done in {}s)", start_instant.elapsed().as_secs());
            return;
        }

        // get new temperature
        let temperature = time_to_temperature(time as f64);

        print!(
            "[time={}]: current_cost={:5.}, current_delay={}, temp={:.2}, ",
            time, current_state.cost, current_state.calculate_total_travel_delay(graph), temperature
        );
        writer
            .write(format!("{},{},{}\n", time, temperature, current_state.cost).as_bytes())
            .unwrap();

        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            print!("-> return");
            println!("(done in {}s)", start_instant.elapsed().as_secs());
            return;
        }

        // get one random overcrowded edge and its occupying groups by index
        let (edge, group_indices) =
            current_state.get_random_overcrowded_edge_with_groups(graph, groups, &mut rng);

        // find a detour for a random group in previously found groups
        let (group_index, path) =
            current_state.find_detour_for_random_group(graph, groups, group_indices, edge, &mut rng);

        
        match path {
            // Another path was found
            Some(path) => {

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
                    steps_without_changes = 0;
                    println!("{}", format!("-> replacing current state").green());
                } else {
                    let probability = (delta_cost as f64 / temperature as f64).exp();
                    let random = rng.gen_range(0.0..1.0);

                    print!("probability={:.2}, random={:.2} ", probability, random);

                    if random < probability {
                        println!("{}", format!("-> choosing worse neighbor").red());
                        current_state = next.clone();
                        if delta_cost == 0 {
                            steps_without_changes += 1;
                        } else {
                            steps_without_changes = 0;
                        }
                    } else {
                        println!("-> skipping");
                        steps_without_changes += 1;
                    }
                }
            },
            // No other path was found
            None => {
                steps_without_changes += 1;
                println!("-> skipping")
            }
        }

        // print!("next_state={:?}, ", next_state.groups_paths_selection);

        time += 1;
    }
}
