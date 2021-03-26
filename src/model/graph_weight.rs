use serde::{Deserialize, Serialize};


/// Node Type of the DiGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimetableNode {
    Departure { // departure of a train ride
        trip_id: u64,
        time: u64,
        station_id: String,
        station_name: String,
    },

    Arrival { // arrival of a train ride
        trip_id: u64,
        time: u64,
        station_id: String,
        station_name: String,
    },

    Transfer { // transfer node at a station, existing for every departure at that station
        time: u64,
        station_id: String,
        station_name: String,
    },

    MainArrival {
        station_id: String,
        station_name: String,
    }
}

impl TimetableNode {

    #[inline]
    pub fn time(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id: _, time, station_id: _, station_name: _} => Some(*time),
            Self::Arrival {trip_id: _, time, station_id: _, station_name: _} => Some(*time),
            Self::Transfer {time, station_id: _, station_name: _} => Some(*time),
            _ => None
        }
    }

    #[inline]
    pub fn station_id(&self) -> Option<String> {
        match self {
            Self::Departure {trip_id: _, time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::Transfer {time: _, station_id, station_name: _} => Some(station_id.clone()),
            Self::MainArrival {station_id, station_name: _} => Some(station_id.clone()),
            _ => None
        }
    }

    #[inline]
    pub fn station_name(&self) -> String {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name} => station_name.clone(),
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name} => station_name.clone(),
            Self::Transfer {time: _, station_id: _, station_name} => station_name.clone(),
            Self::MainArrival { station_id: _, station_name } => station_name.clone()
        }
    }

    #[inline]
    pub fn is_arrival_at_station(&self, target_station_id: &str) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => station_id == target_station_id,
            _ => false
        }
    }

    #[inline]
    pub fn is_departure(&self) -> bool {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    #[inline]
    pub fn is_arrival(&self) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    #[inline]
    pub fn is_transfer(&self) -> bool {
        match self {
            Self::Transfer {time: _, station_id: _, station_name: _}  => true,
            _ => false
        }
    }

    #[inline]
    pub fn is_main_arrival(&self) -> bool {
        match self {
            Self::MainArrival {station_id: _, station_name: _} => true,
            _ => false
        }
    }

    #[inline]
    pub fn kind_as_str(&self) -> &str {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => "Departure",
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => "Arrival",
            Self::Transfer {time: _, station_id: _, station_name: _}  => "Transfer",
            Self::MainArrival {station_id: _, station_name: _} => "MainArrival",
        }
    }

    #[inline]
    pub fn trip_id(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Arrival {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Transfer {time: _, station_id: _, station_name: _}  => None,
            Self::MainArrival {station_id: _, station_name: _} => None,
        }
    }


}


/// Edge Type of the DiGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimetableEdge {
    Trip { // edge between departure and arrival
        duration: u64,
        capacity: u64, // number of passangers that should not be exceeded (but can)
        utilization: u64, // number of passengers on this ride
    },

    WaitInTrain { // edge between arrival and departure in the same train (stay in the train)
        duration: u64
    },
    
    Board, // edge between transfer node and departure

    Alight { // edge between arrival and transfer
        duration: u64
    },

    WaitAtStation { // edge between two transfer nodes
        duration: u64
    },

    Walk { // edge between arrival and next transfer node at other station
        duration: u64
    },

    MainArrivalRelation // connects all arrivals to MainArrival node
}


impl TimetableEdge {

    /// maps edge to some virtual cost for improved DFS (aka. effort/expense to "take" the edge)
    #[inline]
    pub fn travel_cost(&self) -> u64 {
        match self {
            Self::Trip {duration: _, capacity: _, utilization: _} => 2,
            Self::WaitInTrain {duration: _} => 1,
            Self::Alight {duration: _} => 4,
            Self::WaitAtStation {duration: _} => 3,
            Self::Walk {duration: _} => 10,
            Self::Board => 5,
            Self::MainArrivalRelation => 0 // no cost, just a "meta" edge
        }
    }

    /// calculate the utilization cost for edge
    #[inline]
    pub fn utilization_cost(&self) -> u64 {
        match self {

            // penalize utilization over capacity
            Self::Trip {duration: _, capacity, utilization} => {

                if utilization < capacity {
                    0
                } else {
                    // calculate penalty as quadratic diff

                    let diff = *utilization - *capacity;
                    diff.pow(2)
                }
            },

            // for every other edge type return zero
            _ => 0
        }
    }


    /// is RideToStation Edge
    #[inline]
    pub fn is_trip(&self) -> bool {
        match self {
            Self::Trip {
                duration: _, 
                capacity: _, 
                utilization: _
            } => true,
            _ => false,
        }
    }

    /// is WaitInTrain Edge
    #[inline]
    pub fn is_wait_in_train(&self) -> bool {
        match self {
            Self::WaitInTrain {
                duration: _, 
            } => true,
            _ => false,
        }
    }

    /// is Footpath Edge
    #[inline]
    pub fn is_walk(&self) -> bool {
        match self {
            Self::Walk {
                duration: __
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_alight(&self) -> bool {
        match self {
            Self::Alight {
                duration: _
            } => true,
            _ => false
        }
    }

    #[inline]
    pub fn is_wait_at_station(&self) -> bool {
        match self {
            Self::WaitAtStation {
                duration: _
            } => true,
            _ => false
        }
    }

    #[inline]
    pub fn is_board(&self) -> bool {
        match self {
            Self::Board => true,
            _ => false
        }
    }
    
    #[inline]
    pub fn is_main_arrival_relation(&self) -> bool {
        match self {
            Self::MainArrivalRelation => true,
            _ => false
        }
    }

    /// get duration of self, defaults to 0
    #[inline]
    pub fn duration(&self) -> u64 {
        match self {
            Self::Trip{duration, capacity: _, utilization: _} => *duration,
            Self::WaitInTrain{duration} => *duration,
            Self::Alight{duration} => *duration,
            Self::WaitAtStation{duration} => *duration,
            Self::Walk{duration} => *duration,
            _ => 0,
        }
    }

    /// get capacity_soft_limit of self, defaults to MAX
    #[inline]
    pub fn capacity(&self) -> u64 {
        match self {
            Self::Trip{duration: _, capacity, utilization: _} => *capacity,
            _ => std::u64::MAX, // all other edge types are not limited in terms of capacity
        }
    }

    #[inline]
    pub fn increase_utilization(&mut self, addend: u64) {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization += addend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    #[inline]
    pub fn decrease_utilization(&mut self, subtrahend: u64) {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization -= subtrahend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    /// get utilization of self, defaults to 0
    #[inline]
    pub fn utilization(&self) -> u64 {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization,
            _ => 0 // other edges always return 0 utilization as they have unlimited capacity
        }
    }

    // #[inline]
    // pub fn get_remaining_capacity(&self) -> u64 {
    //     match self {
    //         Self::Ride{duration: _, capacity_soft_limit: capacity, capacity_hard_limit: _, utilization} => *capacity - *utilization,
    //         _ => u64::MAX // other edges always return u64::MAX as they have unlimited capacity
    //     }
    // }

    #[inline]
    pub fn kind_as_str(&self) -> &str {
        match self {
            Self::Trip {duration: _, capacity: _, utilization: _}  => "Trip",
            Self::WaitInTrain {duration: _} => "WaitInTrain",
            Self::Board => "Board",
            Self::Alight {duration: _} => "Alight",
            Self::WaitAtStation {duration: _} => "WaitAtStation",
            Self::Walk {duration: _} => "Walk",
            Self::MainArrivalRelation => "MainArrivalRelation"
        }
    }
}
