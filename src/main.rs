use std::{
    env, 
    io::{prelude::*, BufWriter}, 
    fs::File
};

use model::{Model, group::Group};

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
    let create_new_graph = false;
    let dump_model = false;

    let model_option;
    if create_new_graph {
        model_option = Some(Model::with_stations_trips_and_footpaths(&args[1]));
    } else {
        model_option = Some(Model::load_model(model_folder_path));
    }

    let mut model = model_option.unwrap();

    if dump_model {
        Model::dump_model(&model, model_folder_path);
    }

    if args[1].contains("sample") {
        // create dot code only for sample data

        let dot_code = Model::to_dot(&model);

        BufWriter::new(File::create("graph.dot").unwrap()).write(
            dot_code.as_bytes()
        ).unwrap();
    }

    let groups_option;
    if create_new_graph {
        groups_option = Some(model.find_paths(&format!("{}groups.csv", &args[1]), model_folder_path));
    } else {
        groups_option = Some(Group::load_groups(model_folder_path));
    }

    let groups = groups_option.unwrap();

    if dump_model {
        Group::dump_groups(&groups, model_folder_path);
    }

    model.find_solutions(groups, &format!("{}groups.csv", &args[1]), &args[1]);
}
