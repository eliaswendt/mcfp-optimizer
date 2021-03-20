use std::{fs::File, io::{BufWriter, Write}};

use colored::Colorize;
use petgraph::graph::DiGraph;
use rand::Rng;

use super::SelectionState;
use crate::model::{graph_weight::{TimetableEdge, TimetableNode}, group::Group, path::Path};

/// in each iteration generate a random state
///
/// if new state is better than current -> replace current with new
pub fn randomized_best<'a>(graph: &'a mut DiGraph<TimetableNode, TimetableEdge>, groups: &'a Vec<Group>, filepath: &str) -> SelectionState<'a> {

    println!("randomized_best()");

    let mut rng = rand::thread_rng();

    let mut writer = BufWriter::new(
        File::create(filepath).expect(&format!("Could not create file \"{}\"", filepath))
    );

    writer.write("time,cost\n".as_bytes()).unwrap();

    let mut current = SelectionState::generate_random_state(graph, groups);

    for time in 0..10000 {        
        print!("[time={}]: current_cost={},  ", time, current.cost);
        writer.write(format!("{},{}\n", time, current.cost).as_bytes()).unwrap();

        // actually exactly zero, but difficult with float
        if time == 10000 {
            println!("-> return");
            return current;
        }

        let next = current.random_group_neighbor(graph, &mut rng);

        if  next.cost < current.cost {
            current = next;
            println!("{}", format!("-> replacing current state").green());
        } else {
            println!("-> keep current")
        }
    }

    current
}