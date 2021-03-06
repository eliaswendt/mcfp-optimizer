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

    let csv_folder_path = &args[1];
    let model_folder_path = "dump/";
    let create_new_graph = true;
    let dump_model = true;

    let mut model = if create_new_graph {
        println!(
            "creating new model with_stations_trips_and_footpaths({})",
            csv_folder_path
        );
        Model::with_stations_trips_and_footpaths(csv_folder_path)
    } else {
        println!("loading model from {}", model_folder_path);
        Model::load_model(model_folder_path)
    };

    if dump_model {
        println!("dumping model to {}", model_folder_path);
        Model::dump_model(&model, model_folder_path);
    }

    if csv_folder_path.contains("sample") {
        // create dot code only for sample data

        let dot_code = Model::to_dot(&model);

        BufWriter::new(File::create("graph.dot").unwrap())
            .write(dot_code.as_bytes())
            .unwrap();
    }

    let groups = if create_new_graph {
        model.find_paths_for_groups(&format!("{}/groups.csv", csv_folder_path))
    } else {
        Group::load_groups(model_folder_path)
    };

    if dump_model {
        Group::dump_groups(&groups, model_folder_path);
    }

    // optimization::simulated_annealing::optimize_overloaded_graph(&mut model.graph, &groups);
    optimization::randomized_hillclimb::hill_climb_step(&mut model.graph, &groups, 3);
}
