# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
N=<native-trustee-number>
K=<path-to-the-keys-file>
scp "$K" root@$C1:/root/deploy/trustee$N.toml
ssh root@$C1 "k3s kubectl -n step-infra create secret generic trustee$N-keys \
  --from-file=trustee.toml=/root/deploy/trustee$N.toml && \
  shred -u /root/deploy/trustee$N.toml && \
  test ! -e /root/deploy/trustee$N.toml && \
  echo 'Trustee key import complete; temporary key file removed.'"
