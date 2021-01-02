use std::collections::HashMap;

pub struct Trip {
    pub id: u64,
    pub from_station: String,
    pub departure: u64,
    pub to_station: String,
    pub arrival: u64,
    pub capacity: u64
}

impl Trip {
    pub fn from_maps_to_map(trip_maps: &Vec<HashMap<String, String>>) -> HashMap<String, Self> {

        println!("parsing {} trip(s)", trip_maps.len());

        let mut trips_map = HashMap::with_capacity(trip_maps.len());

        for trip_map in trip_maps.iter() {

            let id = trip_map.get("id").unwrap().parse().unwrap();
            let from_station = trip_map.get("from_station").unwrap().clone();
            let to_station = trip_map.get("to_station").unwrap().clone();

            let key = format!("{}_{}->{}", id, from_station, to_station);

            trips_map.insert(key, 
                Self {
                id,
                from_station: from_station,
                departure: trip_map.get("departure").unwrap().parse().unwrap(),
                to_station: to_station,
                arrival: trip_map.get("arrival").unwrap().parse().unwrap(),
                capacity: trip_map.get("capacity").unwrap().parse().unwrap()
            });
        }

        trips_map
    }
}