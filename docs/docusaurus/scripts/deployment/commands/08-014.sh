# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
k3s kubectl -n step-infra scale --replicas=0 deployment/trustee1 deployment/trustee2
k3s kubectl -n step-infra delete secret trustee1-keys trustee2-keys \
  trustee1-auth trustee2-auth
find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 -delete
if _agd_absent=$(k3s kubectl -n step-infra get secret trustee1-keys trustee2-keys 2>&1); then
  printf 'Unexpected trustee key secret remains\n' >&2
  false
else
  printf '%s\n' "$_agd_absent" | grep -c NotFound
fi
unset _agd_absent
find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 | wc -l
