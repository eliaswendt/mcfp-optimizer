use std::collections::HashMap;

pub struct Footpath {
    pub from_station: String,
    pub to_station: String,
    pub duration: u64
}

impl Footpath {
    pub fn from_maps_to_vec(footpath_maps: &Vec<HashMap<String, String>>) -> Vec<Self> {

        let mut footpaths_vec = Vec::with_capacity(footpath_maps.len());

        for footpath_map in footpath_maps.iter() {
            footpaths_vec.push(Self {
                from_station: footpath_map.get("from_station").unwrap().clone(),
                to_station: footpath_map.get("to_station").unwrap().clone(),
                duration: footpath_map.get("duration").unwrap().parse().unwrap()
            });
        }

        footpaths_vec
    }
}