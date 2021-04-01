use std::{env, fs::OpenOptions, io::prelude::*};

use model::{group::Group, Model};
use petgraph::{EdgeDirection::Outgoing, graph::NodeIndex};

mod csv_reader;
mod model;
mod optimization;
use clap::{App, Arg, SubCommand, Values};

/// main entry point of the program, configurable by CLI parameters
///
/// start with `cargo run --release`
///
/// use `--help` to see a list of params
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

        .arg(Arg::with_name("export_as_dot_filepath")
            .short("e")
            .long("export_as_dot_filepath")
            .help("If specified, exports the time-expanded timetable graph as GraphViz DOT-Code to filepath")
            .value_name("FILE"))

        .arg(Arg::with_name("output_folder_path")
            .short("o")
            .long("output")
            .help("folder path for the output CSV file (default='.' aka. current directory)")
            .value_name("FOLDER"))

        .arg(Arg::with_name("search_budgets")
            .short("b")
            .long("search_budgets")
            .help("Specifies a comma-separated list of search budgets each run of the depth-first search is initially provided with.")
            .default_value("30, 35, 40, 45, 50, 55, 60")
            .value_name("LIST<INTEGER>"))

        .arg(Arg::with_name("min_paths")
            .short("p")
            .long("min_paths")
            .help("Specifies the number of paths the iterative-deepening-depth-first search has to find to not retry the DFS with next budget value")
            .default_value("50")
            .value_name("INTEGER"))

        .arg(Arg::with_name("n_search_threads")
            .short("t")
            .long("n_search_threads")
            .help("Specifies the number of threads the program is allowed to spawn for depth-first search of routes through the network.")
            .default_value("1")
            .value_name("INTEGER"))

        .arg(Arg::with_name("n_optimization_iterations_sa1")
            .short("oi")
            .long("n_optimization_iterations_sa1")
            .help("Specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of already discovered routes.")
            .default_value("15000")
            .value_name("INTEGER"))

        .arg(Arg::with_name("n_optimization_iterations_sa2")
            .short("oj")
            .long("n_optimization_iterations_sa2")
            .help("Specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of new routes with interchanged path parts.")
            .default_value("500")
            .value_name("INTEGER"))

        .get_matches();

    // parse config values from cli args
    let input_folder_path_option = matches.value_of("input_folder_path");

    let export_as_dot_option = matches.value_of("export_as_dot_filepath");

    let output_folder_path = matches.value_of("output_folder_path").unwrap_or(".");

    let mut search_budgets: Vec<u64> = matches
        .value_of("search_budgets")
        .unwrap()
        .replace(" ", "")
        .split(',')
        .map(|value| value.parse().expect("search_budgets have to be positive integers"))
        .collect();

    let min_paths: usize = matches
        .value_of("min_paths")
        .unwrap()
        .parse()
        .expect("min_paths has to be a positive integer");

    let n_search_threads: usize = matches
        .value_of("n_search_threads")
        .unwrap()
        .parse()
        .expect("n_search_threads has to be a positive integer");

    let n_optimization_iterations_sa1: u64 = matches
        .value_of("n_optimization_iterations_sa1")
        .unwrap()
        .parse()
        .expect("n_optimization_iterations has to be a positive integer");

    let n_optimization_iterations_sa2: u64 = matches
        .value_of("n_optimization_iterations_sa2")
        .unwrap()
        .parse()
        .expect("n_optimization_iterations has to be a positive integer");




    // EXPLANATION OF input_folder_path:
    // if <input_folder_path> specified, the program will try to read all CSVs from there + create a new model + search paths for all groups + create a snapshot of current model and continue with best path selection
    // if <input_folder_path> is NOT specified, the proram will try to load a snapshot from a previous run and directly continue with best path selection

    let (mut model, groups) = if let Some(input_folder_path) = input_folder_path_option {
        // load model and groups from CSV files

        println!(
            "creating new model with_stations_trips_and_footpaths({}) and groups",
            input_folder_path
        );

        let model = Model::with_stations_trips_and_footpaths(input_folder_path);
        let groups = model
            .find_paths_for_groups(
                &format!("{}/groups.csv", input_folder_path),
                &search_budgets,
                n_search_threads,
                min_paths
        );

        println!("create snapshot of model and groups for next run");
        model.save_to_file();
        Group::save_to_file(&groups);

        (model, groups)
    } else {
        // load model and groups from snpashot

        (
            Model::load_from_file(),
            Group::load_from_file(),
        )
    };

    if let Some(export_as_dot_filepath) = export_as_dot_option {
        // if set, export dot-code of graph to file
        
        println!("exporting dot-code of timetable graph to '{}'", export_as_dot_filepath);
        Model::save_dot_code_to(&model, export_as_dot_filepath);
    }


    let groups_len = groups.len();
    let groups_with_at_least_one_path: Vec<Group> = groups.into_iter().filter(|g| !g.paths.is_empty()).collect();
    
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
    
    // // 1. Optimize with simulated annealing
    let selection_state = optimization::simulated_annealing::simulated_annealing(
        &mut model.graph, 
        &groups_with_at_least_one_path, 
        &format!("{}/simulated_annealing", output_folder_path),
        n_optimization_iterations_sa1
    );

    // save results
    selection_state.save_strained_trip_edges_to_csv(&mut model.graph, &format!("{}/simulated_annealing_edges.csv", output_folder_path));
    selection_state.save_groups_to_csv(&mut model.graph, &format!("{}/simulated_annealing_groups.csv", output_folder_path));

    // // 2. Optimize with simulated annealing on path
    let mut groups_cloned = groups_with_at_least_one_path.clone();
    let selection_state = optimization::simulated_annealing_on_path::simulated_annealing(
        &mut model.graph, 
        &mut groups_cloned, 
        selection_state, 
        &format!("{}/simulated_annealing_on_path", output_folder_path), 
        n_optimization_iterations_sa2
    );

    // save results
    selection_state.save_strained_trip_edges_to_csv(&mut model.graph, &format!("{}/simulated_annealing_on_path_edges.csv", output_folder_path));
    selection_state.save_groups_to_csv(&mut model.graph, &format!("{}/simulated_annealing_on_path_groups.csv", output_folder_path));


    // 3. Optimize with randomized best
    // let selection_state = optimization::randomized_best::randomized_best(
    //     &mut model.graph, 
    //     &groups_with_at_least_one_path, 
    //     10000, 
    //     &format!("{}/randomized_best", output_folder_path), 
    // );
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, &format!("{}/randomized_best_edges.csv", output_folder_path));
    // selection_state.save_groups_to_csv(&mut model.graph, &format!("{}/randomized_best_groups.csv", output_folder_path);


    // 4. Optimize with randomized_hillclimb
    // let selection_state = optimization::randomized_hillclimb::randomized_hillclimb(
    //     &mut model.graph, 
    //     &groups_with_at_least_one_path, 
    //     10,  
    //     10000, 
    //     &format!("{}/randomized_hillclimb", output_folder_path)
    // );
    // selection_state.save_strained_trip_edges_to_csv(&mut model.graph, &format!("{}/randomized_hillclimb_edges.csv", output_folder_path));
    // selection_state.save_groups_to_csv(&mut model.graph, &format!("{}/randomized_hillclimb_groups.csv", output_folder_path));

    println!("done with main() -> terminating")
}


// unused, but too good to go ;)

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



// print first group's path in short 
// selection_state.groups[0].paths[selection_state.groups_path_index[0]].display(&model.graph);


// create subgraph from path of first group
// selection_state.groups[0].paths[selection_state.groups_path_index[0]].create_subgraph_from_edges(&model.graph, "graphs/group_0_selected_path.dot");

// two times
// println!("{}", selection_state.groups[10].paths[selection_state.groups_path_index[10]].to_human_readable_string(&model.graph));
