# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
for host in "$C1" "$C2"; do
  if timeout 3 bash -c 'exec 3<>/dev/tcp/$1/22' -- "$host" 2>/dev/null; then
    printf '%s 22 open\n' "$host"
  else
    printf '%s 22 closed\n' "$host"
  fi
done
