use std::{
    env, 
    io::{prelude::*, BufWriter}, 
    fs::File
};

mod csv_reader;
mod model;
pub mod optimization;

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("run with {} <csv_folder_path>", args[0]);
        return;
    }

    let mut model = model::Model::with_stations_trips_and_footpaths(&args[1]);

    if args[1].contains("sample") {
        // create dot code only for sample data

        let dot_code = model.to_dot();

        BufWriter::new(File::create("graph.dot").unwrap()).write(
            dot_code.as_bytes()
        ).unwrap();
    }

    model.find_solutions(&format!("{}groups.csv", &args[1]));
}
