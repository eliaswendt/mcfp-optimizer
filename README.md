Praktikum Algorithmik
=====================
Dorian Arnouts, Elias Wendt  
Wintersemester 2020/21


## About
This program aims to solve the Multi-Commodity Flow Problem of passenger flow distribution in the railroad network.

The algorithm mainly consists out of three steps:

1. First, a train timetable defined by the input files is read in to build an in-memory time-expanded graph of the connection network.

2. For each travel group defined in the input data, partially-informed depth-first search is used to find several route options for their journey.

3. Last, the algorithm tries to compose an optimal combination of routes (one for each group) using simulated annealing. One goal is to keep the selected route for each group as short and pleasant (few train changes, non-overcrowded trains) as possible, but also not to overload the network in general.


## Input
The Algorithm expects the input separated in four different CSV files stored in the same folder.

Example configuration:
``` 
csv_input_data/
├── footpaths.csv
├── groups.csv
├── stations.csv
└── trips.csv
```

### foothpaths.csv

| field_name   | description                                      |
|--------------|--------------------------------------------------|
| from_station | station id of the footpath's origin station      |
| to_station   | station id of the footpath's destination station |
| duration     | number of minutes taking this footpath takes     |

---
### groups.csv

| field name  | description                                                                                    |
|-------------|------------------------------------------------------------------------------------------------|
| id          | unique identifier of the group                                                                 |
| start       | station id of the station this group wants to travel from                                      |
| departure   | time (in minutes) this group wants to start their journey                                      |
| destination | station id of the station this group want to travel to                                         |
| arrival     | time (in minutes) this group was originally intended to arrive at their destination            |
| passengers  | number of passengers in this group                                                             |
| in_trip     | optional field to specify whether this group wants to start at a station or directly in a trip |

---
### stations.csv

| field name | description                                                                  |
|------------|------------------------------------------------------------------------------|
| id         | unique identifier of the station                                             |
| transfer   | time (in minutes) a passenger requires to alight from a train to the station |
| name       | human-readable name of the station                                           |

---
### trips.csv

Each line the file only describes a fraction / a ride between **two** stations. The whole trip is described by multiple lines.
| field name   | description                                                       |
|--------------|-------------------------------------------------------------------|
| id           | unique identifier of the trip                                     |
| from_station | start station's id of this fraction of the trip                   |
| departure    | time (in minutes) this fraction of the trip start at from_station |
| to_station   | destination station's id of this fraction of the trip             |
| arrival      | time (in minutes) this fraction of the trip ends at to_station    |
| capacity     | number of passengers this trip is able to handle                  |

## Output



## How to build it
This project can be built with Rusts' build tool and package manager Cargo. 
Follow https://www.rust-lang.org/learn/get-started to install it.

Then use the following command to build and run the program:

```
$ cargo build --release
```
The binary can then be found at `target/release/praktikum-algorithmik`

## How to use it
Quick example:
```
# note that <csv_input_folder_path> must not end with a '/'
$ cargo run --release <csv_input_folder_path> <csv_output_filepath> [OPTION]
```

### CLI Parameter OPTIONs
specifies the folder path of the CSV input data

`-b, --search-budget` specifies the search budget each run of the depth-first search is initially provided with (default=60). Too-high budgets can cause **very** long runing times, but too-low values may decrease the number of paths the algorithm can find for each travel-group.

`-t, --threads` specifies the number of threads the program is allowed to spawn for depth-first search of routes through the network (default=1).

`-i, --iterations` specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of routes (default=15000).


`-

## Code Documentation
Browsable code documentation can be generated directly from the source code:
```
# just generate the documentation
$ cargo doc

# generate the documentation and open in (in browser)
$ cargo doc --open
```