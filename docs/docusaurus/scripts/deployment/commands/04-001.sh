# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n kube-system get service traefik \
  -o jsonpath='{.spec.ports[*].port} {.spec.ports[*].nodePort}{"\n"}'
