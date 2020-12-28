use petgraph::dot::{Dot, Config};

mod csv_reader;
mod model;

use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::net::TcpStream;

use std::env;


fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("run with {} <csv_folder_path>", args[0]);
        return;
    }

    let model = model::Model::with_stations_footpaths_and_trips(&args[1]);

    model.find_solutions(&format!("{}/groups.csv", &args[1]), 60);

    // let dot_code = model.to_dot();

    // BufWriter::new(File::create("graphs/graph.dot").unwrap()).write(
    //     dot_code.as_bytes()
    // ).unwrap();
}
