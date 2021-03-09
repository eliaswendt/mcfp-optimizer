use std::thread::current;

use super::SelectionState;
use crate::model::{group::Group, path::Path};

fn time_to_temperature(time: u64) -> u64 {
    10 / time
}

pub fn simulated_annealing<'a>(groups: &'a Vec<Group>) -> SelectionState<'a> {

    let mut current_state = SelectionState::generate_random_state(groups);
    let mut time = 1;

    loop {
        let temperature = time_to_temperature(time);

        if temperature == 0 {
            return current_state;
        }



        time += 1;
    }
}