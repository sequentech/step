#!/bin/bash -i
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail


# To find out the files with most lines of code of a specific extension, for
# example ".ts", to check if it should be excluded, you can execute:
# 
# find . | grep "\.ts$" | xargs wc -l 2>/dev/null | sort -r | head -n 10
#
# If you want to exclude a specific file, you can add it to `.ignore` located
# in the dir where `scc` is executing.

for i in $(find packages/ -maxdepth 1 -type d)
do
    scc -s lines $i
done