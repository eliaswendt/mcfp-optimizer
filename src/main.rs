use std::{
    env,
    fs::File,
    io::{prelude::*, BufWriter},
};

use model::{group::Group, Model};

mod csv_reader;
mod model;
mod optimization;

fn main() {
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
        let groups = model.find_paths_for_groups(&format!("{}/groups.csv", csv_folderpath));

        println!("create snapshot of model and groups for next run");
        Model::save_to_file(&model, snapshot_folder_path);
        Group::save_to_file(&groups, snapshot_folder_path);

        println!("building a graphviz graph of model");
        if csv_folderpath.contains("sample_data") {
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

    // at this state we can start with group's paths selection

    let groups_with_at_least_one_path: Vec<Group> = groups.into_iter().filter(|g| !g.paths.is_empty()).collect();

    // optimization::simulated_annealing::optimize_overloaded_graph(&mut model.graph, &groups);
    optimization::randomized_hillclimb::randomized_hillclimb(&mut model.graph, &groups_with_at_least_one_path, 100,  100);
    // optimization::simulated_annealing_elias::simulated_annealing(&mut model.graph, &groups_with_at_least_one_path);


    println!("done with main() -> terminating")
}
