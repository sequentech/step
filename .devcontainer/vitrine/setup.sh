#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Prepares the development Vitrine services of the compose profile `vitrine`
# (docs/docusaurus/docs/07-developers/14-vitrine.md): the vitrine-log sidecar
# and its two witnesses. Run by the `vitrine-setup` service, with this
# directory at /vitrine, the services' state volume at /state and their data
# volume at /data.
#
# The services refuse key and token files that others can read, and a checkout
# doesn't keep file modes, so this copies the development keys and tokens of
# secrets/ into the state volume, readable by the services' user only. It then
# writes the services' configuration files. The sidecar keeps an event's logs
# in the event's electoral-log database, and its configuration must list each
# of those databases: this lists the ones that exist when it runs and that the
# sidecar's role may use. After creating an election event, run it again and
# restart the sidecar, before enabling the event's transparency.
set -euo pipefail

: "${ELECTORAL_LOG_PG_HOST:?must be set}"
: "${ELECTORAL_LOG_PG_PORT:?must be set}"
: "${ELECTORAL_LOG_PG_DATABASE:?must be set}"
: "${ELECTORAL_LOG_PG_VITRINE_USER:?must be set}"
: "${ELECTORAL_LOG_PG_VITRINE_PASSWORD:?must be set}"
: "${ELECTORAL_LOG_PG_VITRINE_DATABASE:?must be set}"

SOURCE=/vitrine
STATE=/state
DATA=/data
# The user that Vitrine's runtime image runs as.
SERVICE_UID=10001
SERVICE_GID=10001
# The services share the network namespace of the `vitrine` service. The
# public listeners are the ones deployment.toml names; Harvest and Windmill
# reach the internal ones at vitrine:<port>.
LOG_PUBLIC_PORT=8094
LOG_INTERNAL_PORT=8095
WITNESS_PORTS=("8096 8097" "8098 8099")
# The verifier key of secrets/enrolment.key, which signs the logs' enrolment
# notes for the witnesses.
ENROLMENT_VKEY="sequent.localhost/enrol+6263d60c+AaZW2KS7BsI91MGAP0Nufb8tHIX4aYyGkFfw1BMQ9+hJ"

owned_dir() {
    install -d -m 0700 -o "$SERVICE_UID" -g "$SERVICE_GID" "$@"
}

private_copy() {
    install -m 0600 -o "$SERVICE_UID" -g "$SERVICE_GID" "$1" "$2"
}

# Writes stdin to a file of the services' user, replacing it at once.
write_config() {
    local path=$1
    cat >"$path.new"
    chown "$SERVICE_UID:$SERVICE_GID" "$path.new"
    chmod 0644 "$path.new"
    mv "$path.new" "$path"
}

database_url() {
    printf 'postgres://%s:%s@%s:%s/%s' "$ELECTORAL_LOG_PG_VITRINE_USER" \
        "$ELECTORAL_LOG_PG_VITRINE_PASSWORD" "$ELECTORAL_LOG_PG_HOST" \
        "$ELECTORAL_LOG_PG_PORT" "$1"
}

owned_dir "$STATE/log" "$STATE/w1" "$STATE/w2" "$DATA/log" "$DATA/log/keys" \
    "$DATA/w1" "$DATA/w2"
private_copy "$SOURCE/secrets/admin.token" "$STATE/log/admin.token"
private_copy "$SOURCE/secrets/enrolment.key" "$STATE/log/enrolment.key"
for n in 1 2; do
    private_copy "$SOURCE/secrets/witness-$n.key" "$STATE/w$n/witness.key"
    private_copy "$SOURCE/secrets/witness-$n.token" "$STATE/w$n/admin.token"
done

# The election event databases, named <base database>_<event ID as 32 hex
# digits>, that the sidecar's role may connect to and create its schema in.
# An event database created before that role existed gets its grants from
# `electoral-log-admin upgrade-event-databases`.
listed=$(
    PGPASSWORD=$ELECTORAL_LOG_PG_VITRINE_PASSWORD psql -X -At -v ON_ERROR_STOP=1 \
        --host "$ELECTORAL_LOG_PG_HOST" --port "$ELECTORAL_LOG_PG_PORT" \
        --username "$ELECTORAL_LOG_PG_VITRINE_USER" \
        --dbname "$ELECTORAL_LOG_PG_VITRINE_DATABASE" \
        -v base="$ELECTORAL_LOG_PG_DATABASE" <<'SQL'
SELECT datname FROM pg_database
WHERE datname ~ ('^' || :'base' || '_[0-9a-f]{32}$')
  AND has_database_privilege(datname, 'CONNECT')
  AND has_database_privilege(datname, 'CREATE')
ORDER BY datname;
SQL
)
databases=()
if [ -n "$listed" ]; then
    mapfile -t databases <<<"$listed"
fi

{
    cat <<EOF
# Written by .devcontainer/vitrine/setup.sh: edit that script instead.

[server]
public_listen = "0.0.0.0:$LOG_PUBLIC_PORT"
internal_listen = "0.0.0.0:$LOG_INTERNAL_PORT"
public_url = "http://localhost:$LOG_PUBLIC_PORT"
internal_url = "http://vitrine:$LOG_INTERNAL_PORT"
admin_token_file = "$STATE/log/admin.token"
keys_dir = "$DATA/log/keys"
enrolment_key_file = "$STATE/log/enrolment.key"
replica = "log-1"

[database]
url = "$(database_url "$ELECTORAL_LOG_PG_VITRINE_DATABASE")"
pool = 4

[tile_store]
kind = "posix"
path = "$DATA/log/tiles"
EOF
    for database in "${databases[@]}"; do
        printf '\n[databases.%s]\nurl = "%s"\npool = 4\n' "$database" \
            "$(database_url "$database")"
    done
} | write_config "$STATE/log/config.toml"

for n in 1 2; do
    read -r public internal <<<"${WITNESS_PORTS[$((n - 1))]}"
    write_config "$STATE/w$n/config.toml" <<EOF
# Written by .devcontainer/vitrine/setup.sh: edit that script instead.

name = "witness-$n.localhost"
key_file = "$STATE/w$n/witness.key"
listen = "0.0.0.0:$public"
internal_listen = "0.0.0.0:$internal"
database = "$DATA/w$n/witness.db"
admin_token_file = "$STATE/w$n/admin.token"

[limits]
# Every client of the development environment connects from one address.
rate_per_second = 0.0
rate_burst = 1.0

[[enrolment_keys]]
name = "sequent"
vkey = "$ENROLMENT_VKEY"
quota = 1000
EOF
done

echo "Vitrine: the sidecar lists ${#databases[@]} election event database(s)"
