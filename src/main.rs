use std::{
    env, 
    io::{prelude::*, BufWriter}, 
    fs::File
};

use model::group::Group;

mod csv_reader;

mod model;
pub mod optimization;

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("run with {} <csv_folder_path>", args[0]);
        return;
    }

    let model_folder_path = "dump/";
    let create_new_graph = true;

    let mut model = None;
    if create_new_graph {
        model = Some(model::Model::with_stations_trips_and_footpaths(&args[1]));
        model.unwrap().dump_model(model_folder_path);
    } else {
        model = Some(model::Model::load_model(model_folder_path));
    }

    if args[1].contains("sample") {
        // create dot code only for sample data

        let dot_code = model.unwrap().to_dot();

        BufWriter::new(File::create("graph.dot").unwrap()).write(
            dot_code.as_bytes()
        ).unwrap();
    }

    let mut groups = None;
    if create_new_graph {
        groups = Some(model.unwrap().find_paths(&format!("{}groups.csv", &args[1]), model_folder_path));
        Group::dump_groups(groups.unwrap().to_vec(), model_folder_path);
    } else {
        groups = Some(Group::load_groups(model_folder_path));
    }

    model.unwrap().find_solutions(groups.unwrap(), &format!("{}groups.csv", &args[1]), &args[1]);
}
