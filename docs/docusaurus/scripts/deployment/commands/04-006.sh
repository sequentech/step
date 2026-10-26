# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl get nodes -o wide    # c1 and c2 Ready, at the addresses of the site file
k3s kubectl get pods -A          # only kube-system pods, all Running or Completed
