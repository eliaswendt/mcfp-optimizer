use std::{fs::File, io::{BufWriter, Write}, time::Instant};

use colored::Colorize;
use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::Path};

/// maps time to temperature value
fn time_to_temperature(time: f64) -> f64 {
    //(25000.0 - time).powf(1.1)
    25000.0 / time // cost=782, funktioniert schonmal ganz gut
    // 10000.0 - time // funktioniert kaum, trend stimmt aber
}

pub fn simulated_annealing<'a>(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>, filepath: &str) -> SelectionState<'a> {

    println!("simulated_annealing()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath))
    );

    writer.write("time,temperature,cost,edge_cost,delay_cost\n".as_bytes()).unwrap();

    let mut current = SelectionState::generate_random_state(graph, groups);
    //let mut current = SelectionState::generate_state_with_best_path_per_group(graph, groups);
    let mut time = 1;

    let start_instant = Instant::now();

    loop {
        let temperature = time_to_temperature(time as f64);

        print!("[time={}]: current_cost={}, current_edge_cost={}, current_delay={}, temp={:.2}, ", time, current.cost, current.strained_edges_cost, current.travel_delay_cost, temperature);
        writer.write(format!("{},{},{},{},{}\n", time, temperature, current.cost, current.strained_edges_cost, current.travel_delay_cost).as_bytes()).unwrap();

        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            print!("-> return");
            println!(" (done in {}s)", start_instant.elapsed().as_secs());
            return current;
        }

        // select random next state
        // let next_state = &neighbor_states[rng.gen::<usize>() % neighbor_states.len()];
        // let next = current
        //     .all_direct_neighbors(graph)
        //     .into_iter()
        //     .min_by_key(|s| s.cost)
        //     .unwrap();

        let next = current.random_group_neighbor(graph, &mut rng);
  
        // print!("next_state={:?}, ", next_state.groups_paths_selection);

        // if next_state is better than current_state -> delta positive
        // if next_state is worse than current_state -> delta negative
        let delta_cost = current.cost as i64 - next.cost as i64;

        print!("delta_cost={}, ", delta_cost);

        if delta_cost > 0 {
            current = next.clone();
            println!("{}", format!("-> replacing current state").green());
        } else {
            let probability = (delta_cost as f64 / temperature as f64).exp();
            let random = rng.gen_range(0.0..1.0);

            print!("probability={:.2}, random={:.2} ", probability, random);

            if random < probability {
                println!("{}", format!("-> choosing worse neighbor").red());
                current = next.clone();
            } else {
                println!("-> skipping")
            }
        }

        time += 1;
    }
}