use petgraph::dot::{Dot, Config};

mod csv_reader;
mod model;

use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::net::TcpStream;



fn main() {

    let model = model::Model::with_stations_footpaths_and_trips("sample_data/");
    let dot_code = format!("{:?}", Dot::with_config(&model.graph, &[]));

    // 
    BufWriter::new(File::create("graph.dot").unwrap()).write(
        dot_code.as_bytes()
    ).unwrap();

}
