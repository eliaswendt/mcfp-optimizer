#!/bin/bash

cd ../

counter=1
while [ $counter -le 100 ] 
do
    cargo run --release &
    wait $!
    counter=$((counter+1))
done