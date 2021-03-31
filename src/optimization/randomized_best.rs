use std::{fs::File, io::{BufWriter, Write}, time::Instant};

use colored::Colorize;
use petgraph::graph::DiGraph;

use super::SelectionState;
use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group};

/// in each iteration generate a random state
///
/// if new state is better than current -> replace current with new
pub fn randomized_best<'a>(graph: &mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>, iterations: u64, filepath: &str) -> SelectionState<'a> {

    println!("randomized_best()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(format!("{}.{}", filepath, "csv")).expect(&format!("Could not create file \"{}.csv\"", filepath))
    );

    writer
        .write("time,cost,edge_cost,travel_cost,delay_cost\n".as_bytes())
        .unwrap();

    let mut r_writer = BufWriter::new(
        File::create(format!("{}_{}.{}", filepath, "runtime", "csv")).expect(&format!("Could not create file \"{}\"", format!("{}_{}.{}", filepath, "runtime", "csv"))),
    );

    r_writer
        .write("runtime,time\n".as_bytes())
        .unwrap();

    let start_instant = Instant::now();

    // let mut current = SelectionState::generate_random_state(graph, groups)
    let mut current = SelectionState::generate_state_with_best_path_per_group(graph, groups);

    for time in 0..iterations {        
        print!(
            "[time={}]: cost={}, edge_cost={}, travel_cost={}, delay_cost={} ",
            time,
            current.cost,
            current.strained_edges_cost,
            current.travel_cost,
            current.travel_delay_cost,
        );
        writer
            .write(
                format!(
                    "{},{},{},{},{}\n",
                    time,
                    current.cost,
                    current.strained_edges_cost,
                    current.travel_cost,
                    current.travel_delay_cost
                )
                .as_bytes(),
            )
            .unwrap();

        // actually exactly zero, but difficult with float
        if time == iterations {
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
            
            return current;
        }

        let next = current.group_neighbor(graph, &mut rng, None, None);

        if  next.cost < current.cost {
            current = next;
            println!("{}", format!("-> replacing current state").green());
        } else {
            println!("-> keep current")
        }
    }

    print!("-> return");
    println!(" (done in {}s)", start_instant.elapsed().as_secs());

    r_writer
    .write(
        format!(
            "{}s,{}\n",
            start_instant.elapsed().as_secs(),
            iterations
        )
        .as_bytes(),
    )
    .unwrap();

    current
}