# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
P="<ports-printed-on-c1>"
port() { timeout 5 bash -c "</dev/tcp/$1/$2" 2>/dev/null && echo "$1:$2 open" \
  || echo "$1:$2 closed"; }
for p in 22 80 6443 10250 $P; do [ $p = 443 ] || port "$C1" $p; done  # all closed
for p in 22 80 443 6443 10250 $P; do port "$C2" $p; done   # all closed
