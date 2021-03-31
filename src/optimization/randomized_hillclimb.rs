use std::{fs::File, io::{BufWriter, Write}, time::Instant};

use petgraph::graph::DiGraph;

use crate::model::{
    group::Group,
    graph_weight::{TimetableEdge, TimetableNode},
};

use super::SelectionState;

/// perform a single Hill Climbing Step
pub fn randomized_hillclimb<'a>(
    graph: &mut DiGraph<TimetableNode, TimetableEdge>,
    groups: &'a Vec<Group>,
    n_restarts: u64,       // number of "parallel" hill-climb searches
    max_n_iterations: u64, // number of iterations to improve result
    filepath: &str,
) -> SelectionState<'a> {
    println!(
        "randomized_hillclimb(n_runs={}, n_iterations={})",
        n_restarts, max_n_iterations
    );

    let mut writer = BufWriter::new(
        File::create(format!("{}.{}", filepath, "csv")).expect(&format!("Could not create file \"{}.csv\"", filepath))
    );

    writer
        .write("run,iteration,cost,edge_cost,travel_cost,delay_cost\n".as_bytes())
        .unwrap();

    let mut r_writer = BufWriter::new(
        File::create(format!("{}_{}.{}", filepath, "runtime", "csv")).expect(&format!("Could not create file \"{}\"", format!("{}_{}.{}", filepath, "runtime", "csv"))),
    );

    r_writer
        .write("runtime,runs,iterations\n".as_bytes())
        .unwrap();

    let start_instant = Instant::now();

    // from each parallel state save the resulting local maximum as (cost, state)
    let mut local_minima: Vec<SelectionState> = Vec::with_capacity(n_restarts as usize);

    for run in 0..n_restarts {
        // choose random configuration as initial state
        // let mut local_minimum = SelectionState::generate_random_state(graph, groups);
        let mut local_minimum = SelectionState::generate_state_with_best_path_per_group(graph, groups);

        println!(
            "[restart={}/{}]: initial_cost={}, edge_cost={}, travel_cost={}, delay_cost={}",
            run + 1,
            n_restarts,
            local_minimum.cost,
            local_minimum.strained_edges_cost,
            local_minimum.travel_cost,
            local_minimum.travel_delay_cost,
        );

        writer
            .write(
                format!(
                    "{}, {},{},{},{},{}\n",
                    run + 1,
                    n_restarts,
                    local_minimum.cost,
                    local_minimum.strained_edges_cost,
                    local_minimum.travel_cost,
                    local_minimum.travel_delay_cost,
                )
                .as_bytes(),
            )
            .unwrap();

        for j in 0..max_n_iterations {
            // search local maximum from this initial configuration
            // let mut neighbors = local_minimum.generate_group_neighbors(graph); // uses too much memory to properly test it :/
            let best_neighbor = local_minimum.all_direct_group_neighbors(graph).into_iter().flatten().min_by_key(|s| s.cost).unwrap();

            if best_neighbor.cost >= local_minimum.cost {
                // no neighbors found OR best neighbor has higher cost than current local maximum

                println!(
                    "\t[iteration={}/{}]: reached local minimum cost={}, edge_cost={}, travel_cost={}, delay_cost={} ",
                    j + 1,
                    max_n_iterations,
                    local_minimum.cost,
                    local_minimum.strained_edges_cost,
                    local_minimum.travel_cost,
                    local_minimum.travel_delay_cost,
                );
                writer
                .write(
                    format!(
                        "{}, {},{},{},{},{}\n",
                        run + 1,
                        n_restarts,
                        local_minimum.cost,
                        local_minimum.strained_edges_cost,
                        local_minimum.travel_cost,
                        local_minimum.travel_delay_cost,
                    )
                    .as_bytes(),
                )
                .unwrap();

                // as we won't find any better solution -> early exit loop
                break;
            }

            println!(
                "\t[iteration={}]: cost={}, edge_cost={}, travel_cost={}, delay_cost={}",
                j + 1,
                local_minimum.cost,
                local_minimum.strained_edges_cost,
                local_minimum.travel_cost,
                local_minimum.travel_delay_cost,
            );
            writer
                .write(
                    format!(
                        "{}, {},{},{},{},{}\n",
                        run + 1,
                        n_restarts,
                        local_minimum.cost,
                        local_minimum.strained_edges_cost,
                        local_minimum.travel_cost,
                        local_minimum.travel_delay_cost,
                    )
                    .as_bytes(),
                )
                .unwrap();

            // set as new local minimum
            local_minimum = best_neighbor
        }

        local_minima.push(local_minimum);
    }

    local_minima.sort_unstable_by_key(|s| s.cost);
    println!("lowest local minimum: {:?}", local_minima[0].cost);


    // move miminum to end of vec and pop this element
    local_minima.reverse();

    r_writer
        .write(
            format!(
                "{}s,{},{}\n",
                start_instant.elapsed().as_secs(),
                n_restarts,
                max_n_iterations
            )
            .as_bytes(),
        )
        .unwrap();

    return local_minima.pop().unwrap()

    // // stores the index of the currently selected path in each group
    // let mut selected_groups: Vec<usize> = Vec::with_capacity(groups.len());
}
