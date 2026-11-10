# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env && . /root/deploy/secrets.env
kc() { k3s kubectl -n step-infra exec -i deploy/keycloak -- \
  /opt/keycloak/bin/kcadm.sh "$@" --config /tmp/kcadm.config; }
# kcadm takes the password from KC_CLI_PASSWORD, read from standard input, so
# that it is on no command line.
printf '%s\n' "$KC_ADMIN_PASSWORD" | k3s kubectl -n step-infra exec -i deploy/keycloak -- \
  bash -c 'IFS= read -r KC_CLI_PASSWORD && export KC_CLI_PASSWORD &&
    exec /opt/keycloak/bin/kcadm.sh config credentials --server http://localhost:8090 \
      --realm master --user "$1" --config /tmp/kcadm.config' kcadm "$KC_ADMIN_USER"
U='<user-name-at-the-authentication-server>'
jq -n --arg u "$U" --arg m '<e-mail-address>' --arg t "${CR#tenant-}" \
  '{username: $u, email: $m, emailVerified: true, enabled: true,
    groups: ["/admin"], attributes: {"tenant-id": [$t]}}' \
  | kc create users -r "$CR" -f -
for N in 1 2; do
  jq -n --arg n trustee$N --arg v TRUSTEE${N}_PASSWORD --arg t "${CR#tenant-}" \
    '{username: $n, enabled: true, groups: ["/trustee"],
      attributes: {"tenant-id": [$t], trustee: [$n]},
      credentials: [{type: "password", value: $ENV[$v], temporary: false}]}' \
    | kc create users -r "$CR" -f -
done
K=$(kc get clients -r "$CR" -q clientId=api-key-client --fields id \
  --format csv --noquotes)
jq -n '{secret: $ENV.KC_API_KEY_CLIENT_SECRET}' \
  | kc update clients/$K -r "$CR" --merge -f -
kc get users -r "$R" --fields id,username --format csv --noquotes \
  | grep -v ',service-account-' | cut -d, -f1 | while read -r I; do
    kc update users/$I -r "$R" -s enabled=false < /dev/null; done
kc get users -r "$CR" --fields username,enabled --format csv --noquotes
kc get users -r "$R" --fields username,enabled --format csv --noquotes
k3s kubectl -n step-infra exec deploy/keycloak -- rm /tmp/kcadm.config
