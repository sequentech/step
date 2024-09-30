#!/bin/bash
set -x

(( factor = 10000 ))
for (( k = 1; k < 10; ++k )); do
    a=$(( factor*k ))
    ./init.sh 1
    ./dkg.sh
    ./ballots.sh 1 $a
    ./tally.sh
done