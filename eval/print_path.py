import sys
import pandas as pd


def print_path(path: str):
    path_split = path.split("->")
    for s in path_split:
        (location, time, kind) = tuple(s.split("$"))
        if kind == "Arrival" or kind == "Departure":
            print(kind + " at station " + location + ", time=" + time)
        else:
            in_trip = ""
            if location != "":
                in_trip = " in trip " + location
            print("\t" + kind + " with duration " + time + in_trip)


def get_path(group_id: int, filepath: str):
    df = pd.read_csv(filepath, sep="|")
    try:
        path = df[df["group_id"] == group_id]["path"].iloc[0]
    except:
        sys.exit("Group id not found!")
    return path


def __main__():
    if len(sys.argv) < 3:
        sys.exit(
            "Please specify the group number and the input csv file with the path of the groups.")

    group_id = int(sys.argv[1])
    filepath = sys.argv[2]
    path = get_path(group_id, filepath)
    print_path(path)


if __name__ == "__main__":
    __main__()
