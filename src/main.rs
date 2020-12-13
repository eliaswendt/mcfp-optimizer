use petgraph::dot::{Dot, Config};

mod csv_reader;
mod model;


fn main() {

    let model = model::Model::with_stations_footpaths_and_trips("sample_data/");
    println!("{:?}", Dot::with_config(&model.graph, &[Config::EdgeNoLabel]));

}
