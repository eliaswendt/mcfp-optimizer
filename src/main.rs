use std::{
    env,
};

use model::{group::Group, Model};
use optimization::randomized_best::{self, randomized_best};
use optimization::SelectionState;

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

        if csv_folderpath.contains("sample_data") {
            println!("building a graphviz graph of model");
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

    let mut groups_with_at_least_one_path: Vec<Group> = groups.into_iter().filter(|g| !g.paths.is_empty()).collect();

    let avg_paths_per_group = 
        groups_with_at_least_one_path.iter().map(|g| g.paths.len() as u64).sum::<u64>() /
        groups_with_at_least_one_path.len() as u64;

    println!("state-space: {} group(s) with an average of {} path(s) each", groups_with_at_least_one_path.len(), avg_paths_per_group);
    

    // optimization::simulated_annealing::optimize_overloaded_graph(&mut model.graph, &groups);
    // optimization::randomized_hillclimb::randomized_hillclimb(&mut model.graph, &groups_with_at_least_one_path, 100,  100);
    // let mut groups_cloned = groups_with_at_least_one_path.clone();
    let selection_state = optimization::simulated_annealing_elias::simulated_annealing(&mut model.graph, &mut groups_with_at_least_one_path, "eval/simulated_annealing.csv");
    //optimization::randomized_best::randomized_best(&mut model.graph, &groups_with_at_least_one_path, "eval/randomized_best.csv");

    // let selection_state = SelectionState {
    //     groups: &Vec::new(),
    //     cost: 0, //state.cost, //SelectionState::generate_random_state(graph, groups); //state;
    //     groups_path_index: Vec::new() //state.groups_paths_selection
    // };
    // let selection_state = optimization::simulated_annealing_on_path::simulated_annealing(&mut model.graph, &mut groups_with_at_least_one_path, selection_state, "eval/simulated_annealing_on_path.csv");

    println!("Selected State: {}", selection_state);

    selection_state.groups[10].paths[selection_state.groups_path_index[10]].display(&model.graph);

    println!("done with main() -> terminating")
}
