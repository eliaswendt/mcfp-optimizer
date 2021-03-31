use std::{env, fs::OpenOptions, io::prelude::*};

use model::{group::Group, Model};
use petgraph::{EdgeDirection::Outgoing, graph::NodeIndex};

mod csv_reader;
mod model;
mod optimization;
use clap::{Arg, App, SubCommand};

fn main() {

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))

        .arg(Arg::with_name("input_folder_path")
            .short("i")
            .long("input")
            .help("folder path of the input CSV files")
            .value_name("FOLDER"))

        .arg(Arg::with_name("output_folder_path")
            .short("o")
            .long("output")
            .help("folder path for the output CSV file (default='.' aka. current directory)")
            .value_name("FOLDER"))

        .arg(Arg::with_name("search_budget")
            .short("b")
            .long("search_budget")
            .help("Specifies the search budget each run of the depth-first search is initially provided with (default=60).")
            .value_name("INTEGER"))

        .arg(Arg::with_name("n_search_threads")
            .short("t")
            .long("n_search_threads")
            .help("Specifies the number of threads the program is allowed to spawn for depth-first search of routes through the network (default=1).")
            .value_name("INTEGER"))

        .arg(Arg::with_name("n_optimization_iterations")
            .short("i")
            .long("n_optimization_iterations")
            .help("Specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of routes (default=15000).")
            .value_name("INTEGER"))

        .get_matches();

    // parse config values from cli args
    let input_folder_path = matches.value_of("input_folder_path").unwrap();
    let output_folder_path = matches.value_of("output_folder_path").unwrap_or(".");
    let search_budget: usize = matches
        .value_of("search_budget")
        .unwrap_or("60")
        .parse()
        .expect("search_budget has to be a positive number");
    let n_search_threads: usize = matches
        .value_of("n_search_threads")
        .unwrap_or("1")
        .parse()
        .expect("n_search_threads has to be a positive number");
    let n_optimization_iterations: u64 = matches
        .value_of("n_optimization_iterations")
        .unwrap_or("15000")
        .parse()
        .expect("n_optimization_iterations has to be a positive number");




    // EXPLANATION OF CLI ARGUMENT USAGE:
    // if <csv_folderpath> specified, the program will try to read all CSVs from there + create a new model + search paths for all groups + create a snapshot of current model and continue with best path selection
    // if <csv_folderpath> is NOT specified, the proram will try to load a snapshot from a previous run and directly continue with best path selection

    let args: Vec<String> = env::args().collect();
    let csv_folderpath_option = if args.len() != 2 {
        println!("<csv_folderpath> not specified -> trying to load snapshot from last run");
        None
    } else {
        println!("using CSV folderpath \"{}\" to create new graph", args[1]);
        Some(&args[1])
    };

    let snapshot_folder_path = "snapshot/";

    let (mut model, groups) = if let Some(csv_folderpath) = csv_folderpath_option {
        println!(
            "creating new model with_stations_trips_and_footpaths({}) and groups",
            csv_folderpath
        );

        let model = Model::with_stations_trips_and_footpaths(csv_folderpath);
        let groups = model
            .find_paths_for_groups(
                &format!("{}/groups.csv", csv_folderpath),
                &vec![30, 35, 40, 45, 50, 55, 60],
                n_search_threads
        );

        println!("create snapshot of model and groups for next run");
        model.save_to_file(snapshot_folder_path);
        Group::save_to_file(&groups, snapshot_folder_path);

        if csv_folderpath.contains("sample_data") {
            println!("building a graphviz graph of model");
            // create dot code only for sample data
            Model::save_dot_code_to(&model, &format!("{}/graph.dot", csv_folderpath));
        }

        (model, groups)
    } else {
        (
            Model::load_from_file(snapshot_folder_path),
            Group::load_from_file(snapshot_folder_path),
        )
    };

    let groups_len = groups.len();
    let mut groups_with_at_least_one_path: Vec<Group> = groups.into_iter().filter(|g| !g.paths.is_empty()).collect();
    
    let avg_paths_per_group = 
    groups_with_at_least_one_path.iter().map(|g| g.paths.len() as u64).sum::<u64>() /
    groups_with_at_least_one_path.len() as u64;
    
    // at this state we can start with group's paths selection
    println!(
        "state-space: {} group(s) with an average of {} path(s) each\n{} groups ({}%) without known path", 
        groups_with_at_least_one_path.len(), 
        avg_paths_per_group,
        groups_len - groups_with_at_least_one_path.len(),
        100 * (groups_len - groups_with_at_least_one_path.len()) / groups_len
    );
    
    // ELIAS
    // create and save a graph of all groups combined possible paths
    // for group in groups_with_at_least_one_path.iter() {
    //     let edges = group.paths
    //         .iter()
    //         .map(|path| path.edges.iter())
    //         .flatten()
    //         .cloned()
    //         .collect();

    //     model.create_subgraph_from_edges(edges, &format!("graphs/groups/group_{}.dot", group.id));
    // }
    //optimization::analyze_neighborhood(&mut model.graph, &groups_with_at_least_one_path, "eval/benchmark_neighbors/", 10);

    // // 1. Optimize with simulated annealing
    let selection_state = optimization::simulated_annealing::simulated_annealing(
        &mut model.graph, 
        &groups_with_at_least_one_path, 
        "eval/simulated_annealing",
        n_optimization_iterations
    );
    // save results
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, "eval/simulated_annealing_edges.csv");
    // selection_state.save_groups_to_csv(&mut model.graph, "eval/simulated_annealing_groups.csv");

    // let mut file = OpenOptions::new()
    //     .write(true)
    //     .append(true)
    //     .open("eval/simulated_annealing_100_runs.csv")
    //     .unwrap();

    // if let Err(e) = 
    //     writeln!(file, "{},{},{},{}", 
    //         selection_state.cost, 
    //         selection_state.strained_edges_cost, 
    //         selection_state.travel_cost, 
    //         selection_state.travel_delay_cost
    //     ) {
    //         eprintln!("Couldn't write to file: {}", e);
    //     }

    // // 2. Optimize with simulated annealing on path
    let mut groups_cloned = groups_with_at_least_one_path.clone();
    let selection_state = optimization::simulated_annealing_on_path::simulated_annealing(&mut model.graph, &mut groups_cloned, selection_state, "eval/simulated_annealing_on_path");
    // save results
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, "eval/simulated_annealing_on_path_edges.csv");
    // selection_state.save_groups_to_csv(&mut model.graph, "eval/simulated_annealing_on_path_groups.csv");

    // let mut file = OpenOptions::new()
    //     .write(true)
    //     .append(true)
    //     .open("eval/simulated_annealing_on_path_100_runs.csv")
    //     .unwrap();

    // if let Err(e) = 
    //     writeln!(file, "{},{},{},{}", 
    //         selection_state.cost, 
    //         selection_state.strained_edges_cost, 
    //         selection_state.travel_cost, 
    //         selection_state.travel_delay_cost
    //     ) {
    //         eprintln!("Couldn't write to file: {}", e);
    //     }


    // 3. Optimize with randomized best
    // let selection_state = optimization::randomized_best::randomized_best(&mut model.graph, &groups_with_at_least_one_path, 10000, "eval/randomized_best");
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, "eval/randomized_best_edges.csv");
    // selection_state.save_groups_to_csv(&mut model.graph, "eval/randomized_best_groups.csv");


    // 4. Optimize with randomized_hillclimb
    // let selection_state = optimization::randomized_hillclimb::randomized_hillclimb(&mut model.graph, &groups_with_at_least_one_path, 10,  10000, "eval/randomized_hillclimb");
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, "eval/randomized_hillclimb_edges.csv");
    // selection_state.save_groups_to_csv(&mut model.graph, "eval/randomized_hillclimb_groups.csv");

    
    // print first group's path in short 
    // selection_state.groups[0].paths[selection_state.groups_path_index[0]].display(&model.graph);
    
    // two times
    // println!("{}", selection_state.groups[10].paths[selection_state.groups_path_index[10]].to_human_readable_string(&model.graph));

    // create subgraph from path of first group
    // selection_state.groups[0].paths[selection_state.groups_path_index[0]].create_subgraph_from_edges(&model.graph, "graphs/group_0_selected_path.dot");


    println!("done with main() -> terminating")
}
