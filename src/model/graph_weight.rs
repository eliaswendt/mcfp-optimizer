use serde::{Deserialize, Serialize};


/// Node Type of the DiGraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimetableNode {
    Departure { // departure of a train ride
        trip_id: u64,
        time: u64,
        station_id: u64,
        station_name: String,
    },

    Arrival { // arrival of a train ride
        trip_id: u64,
        time: u64,
        station_id: u64,
        station_name: String,
    },

    Transfer { // transfer node at a station, existing for every departure at that station
        time: u64,
        station_id: u64,
        station_name: String,
    }
}

impl TimetableNode {

    /// returns time for node
    #[inline]
    pub fn time(&self) -> u64 {
        match self {
            Self::Departure {trip_id: _, time, station_id: _, station_name: _} => *time,
            Self::Arrival {trip_id: _, time, station_id: _, station_name: _} => *time,
            Self::Transfer {time, station_id: _, station_name: _} => *time,
        }
    }

    /// returns station id for node
    #[inline]
    pub fn station_id(&self) -> u64 {
        match self {
            Self::Departure {trip_id: _, time: _, station_id, station_name: _} => *station_id,
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => *station_id,
            Self::Transfer {time: _, station_id, station_name: _} => *station_id,
        }
    }

    /// returns station name for node
    #[inline]
    pub fn station_name(&self) -> String {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name} => station_name.clone(),
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name} => station_name.clone(),
            Self::Transfer {time: _, station_id: _, station_name} => station_name.clone(),
        }
    }

    /// returns trip id for node
    #[inline]
    pub fn trip_id(&self) -> Option<u64> {
        match self {
            Self::Departure {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Arrival {trip_id, time: _, station_id: _, station_name: _} => Some(*trip_id),
            Self::Transfer {time: _, station_id: _, station_name: _}  => None,
        }
    }

    /// returns true if node is Arrival and its station id equals target_station_id
    #[inline]
    pub fn is_arrival_at_station(&self, target_station_id: u64) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id, station_name: _} => *station_id == target_station_id,
            _ => false
        }
    }

    /// returns true if node is Departure
    #[inline]
    pub fn is_departure(&self) -> bool {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    /// returns true if node is Arrival
    #[inline]
    pub fn is_arrival(&self) -> bool {
        match self {
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => true,
            _ => false
        }
    }

    /// returns true if node is Transfer
    #[inline]
    pub fn is_transfer(&self) -> bool {
        match self {
            Self::Transfer {time: _, station_id: _, station_name: _}  => true,
            _ => false
        }
    }

    /// returns type as string for node
    #[inline]
    pub fn kind_as_str(&self) -> &str {
        match self {
            Self::Departure {trip_id: _, time: _, station_id: _, station_name: _} => "Departure",
            Self::Arrival {trip_id: _, time: _, station_id: _, station_name: _} => "Arrival",
            Self::Transfer {time: _, station_id: _, station_name: _}  => "Transfer",
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
}


impl TimetableEdge {

    /// maps edge to some virtual cost for improved DFS (aka. effort/expense to "take" the edge)
    #[inline]
    pub fn travel_cost(&self) -> u64 {
        match self {
            Self::Trip {duration, capacity: _, utilization: _} => 5 / (*duration + 1),
            Self::WaitInTrain {duration: _} => 0,
            Self::Alight {duration: _} => 6,
            Self::WaitAtStation {duration} => *duration,
            Self::Walk {duration: _} => 6,
            Self::Board => 0,
        }
    }

    /// calculates the utilization cost for edge
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


    /// returns true if edge is Trip
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

    /// returns true if edge is WaitInTrain
    #[inline]
    pub fn is_wait_in_train(&self) -> bool {
        match self {
            Self::WaitInTrain {
                duration: _, 
            } => true,
            _ => false,
        }
    }

    /// returns true if edge is Walk
    #[inline]
    pub fn is_walk(&self) -> bool {
        match self {
            Self::Walk {
                duration: __
            } => true,
            _ => false,
        }
    }

    /// returns true if edge is Alight
    #[inline]
    pub fn is_alight(&self) -> bool {
        match self {
            Self::Alight {
                duration: _
            } => true,
            _ => false
        }
    }

    /// returns true if edge is WaitAtStation
    #[inline]
    pub fn is_wait_at_station(&self) -> bool {
        match self {
            Self::WaitAtStation {
                duration: _
            } => true,
            _ => false
        }
    }

    /// returns true if edge is Board
    #[inline]
    pub fn is_board(&self) -> bool {
        match self {
            Self::Board => true,
            _ => false
        }
    }

    /// returns duration of self, defaults to 0
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

    /// returns capacity_soft_limit of self, defaults to MAX
    #[inline]
    pub fn capacity(&self) -> u64 {
        match self {
            Self::Trip{duration: _, capacity, utilization: _} => *capacity,
            _ => std::u64::MAX, // all other edge types are not limited in terms of capacity
        }
    }

    /// increases utilization of self if self is Trip
    #[inline]
    pub fn increase_utilization(&mut self, addend: u64) {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization += addend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    /// decreases utilization of self if self is Trip
    #[inline]
    pub fn decrease_utilization(&mut self, subtrahend: u64) {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization -= subtrahend,
            _ => {} // no need to track utilization on other edges, as they have unlimited capacity
        }
    }

    /// returns utilization of self if self is Trip, defaults to 0
    #[inline]
    pub fn utilization(&self) -> u64 {
        match self {
            Self::Trip{duration: _, capacity: _, utilization} => *utilization,
            _ => 0 // other edges always return 0 utilization as they have unlimited capacity
        }
    }

    /// returns type as string for edge
    #[inline]
    pub fn kind_as_str(&self) -> &str {
        match self {
            Self::Trip {duration: _, capacity: _, utilization: _}  => "Trip",
            Self::WaitInTrain {duration: _} => "WaitInTrain",
            Self::Board => "Board",
            Self::Alight {duration: _} => "Alight",
            Self::WaitAtStation {duration: _} => "WaitAtStation",
            Self::Walk {duration: _} => "Walk",
        }
    }
}
