# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
# hard coded for trustees = 3
(bash -c "cd ./demo/1 && ./run.sh") > log1.txt 2>&1 &
(bash -c "cd ./demo/2 && ./run.sh") > log2.txt 2>&1 &
(bash -c "cd ./demo/3 && ./run.sh") > log3.txt 2>&1 &
wait
