use std::{collections::{HashMap, LinkedList}, io::prelude::*};
use std::io::BufReader;
use std::fs::File;


type Record = HashMap<String, String>;

pub fn read_to_maps(filepath: &str) -> LinkedList<HashMap<String, String>> {

    let f = File::open(filepath).unwrap();
    let reader = BufReader::new(f);

    let mut rows = LinkedList::new();

    // Build the CSV reader and iterate over each record.
    let mut rdr = csv::Reader::from_reader(reader);
    for (row_index, result) in rdr.deserialize().enumerate() {
        // The iterator yields Result<StringRecord, Error>, so we check the error here.
        let row: Record = result.expect(&format!("Failed to read record in line {}", row_index));
        rows.push_back(row);
    }

    rows
}