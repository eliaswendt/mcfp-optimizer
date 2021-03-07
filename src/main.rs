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
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("run with {} <csv_folder_path>", args[0]);
        return;
    }

    // configuration of this execution
    let csv_folder_path = &args[1];
    let dump_folder_path = "dump/";
    let build_new_model = false;
    let save_model_to_file = true;

    let mut model = if build_new_model {
        println!(
            "creating new model with_stations_trips_and_footpaths({})",
            csv_folder_path
        );
        Model::with_stations_trips_and_footpaths(csv_folder_path)
    } else {
        Model::load_from_file(dump_folder_path)
    };

    if save_model_to_file {
        Model::save_to_file(&model, dump_folder_path);
    }

    if csv_folder_path.contains("sample") {
        // create dot code only for sample data
        Model::save_dot_code(&model, "graph.dot");
    }

    let groups = if build_new_model {
        model.find_paths_for_groups(&format!("{}/groups.csv", csv_folder_path))
    } else {
        Group::load_from_file(dump_folder_path)
    };

    if save_model_to_file {
        Group::save_to_file(&groups, dump_folder_path);
    }

    // optimization::simulated_annealing::optimize_overloaded_graph(&mut model.graph, &groups);
    optimization::randomized_hillclimb::randomized_hillclimb(&mut model.graph, &groups, 30, 100);
}
