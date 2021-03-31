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

pub fn simulated_annealing<'a>(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &'a Vec<Group>,
    filepath: &str,
    n_iterations: u64,
) -> SelectionState<'a> {
    println!("simulated_annealing()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(format!("{}.{}", filepath, "csv"))
            .expect(&format!("Could not create file \"{}.csv\"", filepath)),
    );

    writer
        .write("time,temperature,cost,edge_cost,travel_cost,delay_cost\n".as_bytes())
        .unwrap();

    let mut r_writer = BufWriter::new(
        File::create(format!("{}_{}.{}", filepath, "runtime", "csv")).expect(&format!(
            "Could not create file \"{}\"",
            format!("{}_{}.{}", filepath, "runtime", "csv")
        )),
    );

    r_writer.write("runtime,time\n".as_bytes()).unwrap();

    //let mut current = SelectionState::generate_random_state(graph, groups);
    let mut current = SelectionState::generate_state_with_best_path_per_group(graph, groups);
    let mut time = 1;

    let start_instant = Instant::now();

    loop {
        let temperature = n_iterations as f64 / time as f64; // time-to-temperature mapping

        print!(
            "[time={}]: cost={}, edge_cost={}, travel_cost={}, delay_cost={}, temp={:.2}, ",
            time,
            current.cost,
            current.strained_edges_cost,
            current.travel_cost,
            current.travel_delay_cost,
            temperature
        );
        writer
            .write(
                format!(
                    "{},{},{},{},{},{}\n",
                    time,
                    temperature,
                    current.cost,
                    current.strained_edges_cost,
                    current.travel_cost,
                    current.travel_delay_cost
                )
                .as_bytes(),
            )
            .unwrap();

        // actually exactly zero, but difficult with float
        if temperature < 1.0 {
            print!("-> return");
            println!(" (done in {}s)", start_instant.elapsed().as_secs());

            r_writer
                .write(format!("{}s,{}\n", start_instant.elapsed().as_secs(), time).as_bytes())
                .unwrap();

            return current;
        }

        let next = current.random_group_neighbor(graph, &mut rng, None, None);

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
