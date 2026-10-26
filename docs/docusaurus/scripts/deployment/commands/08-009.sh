# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
(
set +x
set -e
umask 077
. /root/deploy/hosts.env && . /root/deploy/secrets.env
N=<native-trustee-number>
T='<registered-trustee-name>'
case "$N" in 1|2) ;; *) exit 1 ;; esac
[ -n "$T" ]
MATCHES=$(k3s kubectl -n step-infra exec -i deploy/postgres -- \
  psql -U postgres -tA -d hasura -v ON_ERROR_STOP=1 \
  -v tenant="${CR#tenant-}" -v trustee="$T" <<'SQL'
SELECT count(*) FROM sequent_backend.trustee
WHERE tenant_id = :'tenant' AND name = :'trustee';
SQL
)
[ "$MATCHES" = 1 ]
W=$(mktemp -d /root/deploy/.trustee-map.XXXXXXXX)
trap 'rm -rf "$W"; k3s kubectl -n step-infra exec deploy/keycloak -- rm -f /tmp/kcadm-trustee-map.config >/dev/null 2>&1 || true' EXIT
kc() { k3s kubectl -n step-infra exec -i deploy/keycloak -- \
  /opt/keycloak/bin/kcadm.sh "$@" --config /tmp/kcadm-trustee-map.config; }
kc config credentials --server http://localhost:8090 --realm master \
  --user "$KC_ADMIN_USER" --password "$KC_ADMIN_PASSWORD" < /dev/null > "$W/auth.log" 2>&1
kc get users -r "$CR" -q username="trustee$N" -q exact=true \
  --fields id,username < /dev/null > "$W/users.json"
I=$(jq -er --arg n "trustee$N" '[.[] | select(.username==$n)] |
  if length==1 then .[0].id else error("Expected one native account") end' "$W/users.json")
kc get "users/$I" -r "$CR" < /dev/null > "$W/before.json"
jq --arg t "$T" '{attributes:(.attributes + {trustee:[$t]})}' \
  "$W/before.json" > "$W/binding.json"
kc update "users/$I" -r "$CR" -f - < "$W/binding.json" > "$W/update.log" 2>&1
kc get "users/$I" -r "$CR" < /dev/null > "$W/after.json"
jq -e --slurpfile before "$W/before.json" --slurpfile expected "$W/binding.json" \
  '.attributes==$expected[0].attributes and
   (del(.attributes)==($before[0]|del(.attributes)))' "$W/after.json" > /dev/null
echo "Native trustee binding verified"
)
