use std::collections::HashMap;

pub struct Footpath {
    from_station: String,
    to_station: String,
    duration: u64
}

impl Footpath {
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self {
            from_station: map.get("from_station").unwrap().clone(),
            to_station: map.get("to_station").unwrap().clone(),
            duration: map.get("duration").unwrap().parse().unwrap()
        }
    }
}