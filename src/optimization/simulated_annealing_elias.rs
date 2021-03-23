use std::{fs::File, io::{BufWriter, Write}};

use colored::Colorize;
use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::Path};

/// maps time to temperature value
fn time_to_temperature(time: f64) -> f64 {
    // (100000.0 - time).powf(1.1)
    10000.0 / time // cost=782, funktioniert schonmal ganz gut
    // 10000.0 - time // funktioniert kaum, trend stimmt aber
}

pub fn simulated_annealing<'a>(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>, filepath: &str) -> SelectionState<'a> {

    println!("simulated_annealing()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath))
    );

    writer.write("time,temperature,cost\n".as_bytes()).unwrap();

    let mut current = SelectionState::generate_random_state(graph, groups);
    let mut time = 1;

    loop {
        let temperature = time_to_temperature(time as f64);
        
        print!("[time={}]: current_cost={}, current_delay={}, temp={}, ", time, current.cost, current.calculate_total_travel_delay(graph), temperature);
        writer.write(format!("{},{},{}\n", time, temperature, current.cost).as_bytes()).unwrap();

        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            println!("-> return");
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

            print!("probability={}, random={} ", probability, random);

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