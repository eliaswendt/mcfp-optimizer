use std::collections::HashMap;

/// travel group
pub struct Group {
    id: u64,
    
    pub start: String, // Start-Halt für die Alternativensuche (Station ID)
    pub destination: String, // Ziel der Gruppe (Station ID)

    pub departure: u64, // Frühstmögliche Abfahrtszeit am Start-Halt (Integer)
    pub arrival: u64, // Ursprünglich geplante Ankunftszeit am Ziel (Integer)

    pub passengers: usize, // Größe der Gruppe (Integer)


    // Hier gibt es zwei Möglichkeiten (siehe auch unten):
    // Wenn der Wert leer ist, befindet sich die Gruppe am Start-Halt.
    // Wenn der Wert nicht leer ist, gibt er die Trip ID (Integer) der Fahrt an, in der sich die Gruppe befindet.
    pub in_trip: Option<usize>,
}

impl Group {
    pub fn from_maps_to_map(group_maps: &Vec<HashMap<String, String>>) -> HashMap<u64, Self> {

        println!("parsing {} groups", group_maps.len());

        let mut groups_map = HashMap::with_capacity(group_maps.len());

        for group_map in group_maps.iter() {
            let id = group_map.get("id").unwrap().parse().unwrap();

            let in_trip_value = group_map.get("in_trip").unwrap();
            let in_trip = if in_trip_value.is_empty() {
                None
            } else {
                Some(in_trip_value.parse().unwrap())
            };

            groups_map.insert(id, Self {
                id,
                start: group_map.get("start").unwrap().clone(),
                destination: group_map.get("destination").unwrap().clone(),
                departure: group_map.get("departure").unwrap().parse().unwrap(),
                arrival: group_map.get("arrival").unwrap().parse().unwrap(),
                passengers: group_map.get("passengers").unwrap().parse().unwrap(),
                in_trip
            });
        } 

        groups_map
    }
}
