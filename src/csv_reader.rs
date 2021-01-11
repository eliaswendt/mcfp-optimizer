use std::{
    collections::HashMap, 
    io::BufReader, 
    fs::File
};

type Record = HashMap<String, String>;

pub fn read_to_maps(filepath: &str) -> Vec<HashMap<String, String>> {

    let reader = BufReader::new(
        File::open(filepath).expect(&format!("Could not open file {}", filepath))
    );

    let mut rows: Vec<Record> = Vec::new();

    // Build the CSV reader and iterate over each record.
    let mut csv_reader = csv::Reader::from_reader(reader);
    for (row_index, result) in csv_reader.deserialize().enumerate() {
        // The iterator yields Result<StringRecord, Error>, so we check the error here.
        rows.push(
            result.expect(&format!("Failed to read record in line {}", row_index))
        );
    }

    rows
}