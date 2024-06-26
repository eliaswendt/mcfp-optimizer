Multi-Commodity Flow Optimizer
==============================
Dorian Arnouts, Elias Wendt

Winter term 2020/21

@ FG Algorithmik, TU-Darmstadt

## About
This program aims to solve the Multi-Commodity Flow Problem of passenger flow distribution in the railroad network.

The algorithm mainly consists out of three steps:

1. First, a train timetable defined by the input files is read in to build an in-memory time-expanded graph of the connection network.

2. For each travel group defined in the input data, partially-informed depth-first search is used to find several route options for their journey.

3. Last, the algorithm tries to compose an optimal combination of routes (one for each group) using simulated annealing. One goal is to keep the selected route for each group as short and pleasant (few train changes, non-overcrowded trains) as possible, but also not to overload the network in general. In the first part, simulated annealing is used to interchange already found paths. In the second part, it tries to detour groups from overcrowded edges by finding subpaths avoiding this edge.

<br>

### Graph scheme
![timetable graph](timetable.png "Graph scheme")

### Sample timetable graph
![sample_data graph](graph.png "Sample data graph")

## Input
The Algorithm expects the input separated in four different CSV files stored in the same folder.
``` 
<csv_input_folder_path>/
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

<br>

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

<br>

### stations.csv

| field name | description                                                                  |
|------------|------------------------------------------------------------------------------|
| id         | unique identifier of the station                                             |
| transfer   | time (in minutes) a passenger requires to alight from a train to the station |
| name       | human-readable name of the station                                           |

<br>

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
``` 
<csv_output_filepath>/
├── simulated_annealing.csv
├── simulated_annealing_edges.csv
├── simulated_annealing_groups.csv
├── simulated_annealing_runtime.csv
├── simulated_annealing_on_path.csv
├── simulated_annealing_on_path_edges.csv
├── simulated_annealing_on_path_groups.csv
└── simulated_annealing_on_path_runtime.csv
```

### simulated_annealing\<_on_path\>.csv

| field_name  | description                                                    |
|-------------|----------------------------------------------------------------|
| time        | current iteration                                              |
| temperature | temperature of current iteration                               |
| cost        | total cost of current selected state                           |
| edge_cost   | cost of strained edges of current selected state               |
| travel_cost | summed travel cost of selected paths of current selected state |
| delay_cost  | summed delay of selected paths of current selected state       |

<br>

### simulated_annealing\<_on_path\>_edges.csv

Only strained edges, i.e. only trip edges or wait in train edges.

| field_name  | description                                                   |
|-------------|---------------------------------------------------------------|
| edge_index  | index of the edge in graph (identifier)                       |
| duration    | duration of the edge, i.e. travel time, or wait in train time |
| capacity    | capacity of the edge, i.e. capacity in the train              |
| utilization | utilization of the edge, i.e. utilization in the train        |

<br>

### simulated_annealing\<_on_path\>_groups.csv

| field_name   | description                                                                                                                                                                               |
|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| group_id     | unique identifier of the group                                                                                                                                                            |
| planned_time | planned duration (in minutes) from start to destination                                                                                                                                   |
| real_time    | real duration (in minutes) from start to destination with selected path                                                                                                                   |
| travel_cost  | travel cost of path, i.e. summed cost of edges                                                                                                                                            |
| delay        | delay of travel (in minutes)                                                                                                                                                              |
| delay_in_%   | delay of travel in percentage with regard to planned duration                                                                                                                             |
| waiting_time | time waiting at stations for selected path                                                                                                                                                |
| in_trip_time | time sitting in a train for selected path                                                                                                                                                 |
| walks        | number of walks in selected path                                                                                                                                                          |
| path         | the shortened selected path with arrival/destination nodes and walk/trip edges seperated by '->' <br>the nodes are encoded as 'station_name\$time\$kind' <br>the edges are encoded as 'trip_id\$time\$kind |

<br>

### simulated_annealing\<_on_path\>_runtime.csv

| field_name   | description                            |
|--------------|----------------------------------------|
| runtime      | runtime of the alogorithm (in seconds) |
| time         | number of iterations                   |

## How to build it
This project can be built with Rust's build tool and package manager `Cargo`. 
Follow https://www.rust-lang.org/learn/get-started to install it.

Then use the following command to build and run the program:

```
$ cargo build --release
```
The binary can then be found at `target/release/praktikum-algorithmik`.

## How to use it
Quick example:
```
# note that <csv_input_folder_path> must not end with a '/'
# also note the `--` after `--release`: mitigates passing the args to cargo

$ cargo run --release -- -i <csv_input_folder_path> [OPTION]
```

### CLI Parameter OPTIONs
`-i, --input` specifies the folder path of the CSV input data. **Required** if not working on a snapshot from previous run.

`-e, --export_as_dot` If specified, exports the time-expanded timetable graph as GraphViz DOT-Code to filepath.

`-o, --output_folder` specifies the folder the result CSV will be written to (default="." aka. current working dir).

`-b, --search_budgets` specifies the list of search budgets each run of the iterative-deepening-depth-first search is initially provided with (default='30, 35, 40, 45, 50, 55, 60'). IDDFS start with the first budget value for the first iteration and continues probing further budgets, if the search did not return enough routes. Too-high budgets can cause **very** long running times, but too-low values may decrease the number of paths the algorithm can find for each travel-group.

`-p, --min_paths` specifies the number of paths the iterative-deepening-depth-first search has to find to not retry the DFS with next budget value (default=50). 

Example: For ` --search_budgets 30 35 40 [...]` and `--min_paths 50` the program will first try the DFS path search with a budget of `30`. If this search did return at least `50` paths, the search would start again with a budget of `35`, etc.

`-t, --n_search_threads` specifies the number of threads the program is allowed to spawn for depth-first search of routes through the network (default=1).

`-oi, --n_optimization_iterations_sa1` specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of already found routes (default=15000).

`-oj, --n_optimization_iterations_sa2` specifies the number of iterations simulated annealing is allowed to spend finding an optimal combination of new routes with interchanged path parts (default=500).

### Snapshots
For quickly testing different optimization parameters, the program automatically generates a snapshot of its current state right after the depth-first search of group routes. This snapshot is saved in two files `snapshot_model.bincode` and `snapshot_groups.bincode`. Although these are two separated files, they strongly depend on each other and **can not be interchanged with snapshot files of other runs**.

To restart path combination optimization with paths of an earlier run, simply call the program without specifying `-i, --input` parameter.


## Code Overview
Browsable code overview can be generated directly from the source code:
```
# just generate the html doc directory
$ cargo doc

# generate the html doc directory and open it in browser
$ cargo doc --open
```
