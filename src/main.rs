use std::{env, io::{prelude::*, BufWriter}, fs::File};

use petgraph::dot::{Dot, Config};
mod csv_reader;

mod model;


fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("run with {} <csv_folder_path>", args[0]);
        return;
    }

    let mut model = model::Model::with_stations_footpaths_and_trips(&args[1]);

    model.validate_graph_integrity();

    model.find_solutions(&format!("{}groups.csv", &args[1]));


    if args[1].contains("sample") {
        // create dot code only for sample data

        let dot_code = model.to_dot();

        BufWriter::new(File::create("graph.dot").unwrap()).write(
            dot_code.as_bytes()
        ).unwrap();
    }
}
