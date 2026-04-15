#!/bin/sh
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Fetches the CA bundle from harvest before starting nginx, then polls for
# changes and reloads nginx when the bundle is updated.

set -e

CA_FILE=/etc/nginx/client-ca/client-ca.pem
CA_TMP=/tmp/client-ca-new.pem
HARVEST_CA_URL="http://${HARVEST_DOMAIN}/certificate-authorities/pem"
# POLL_INTERVAL=60
POLL_INTERVAL=${KC_SPI_TRUSTSTORE_URL_REFRESH_INTERVAL_SECONDS:-60}

mkdir -p "$(dirname "$CA_FILE")"

echo "Waiting for harvest CA bundle at ${HARVEST_CA_URL} ..."
until wget -qO "$CA_FILE" "$HARVEST_CA_URL" 2>/dev/null; do
    echo "harvest not ready yet, retrying in 5s..."
    sleep 5
done
echo "CA bundle fetched."

# Poll for CA changes in the background after nginx has started.
poll_and_reload() {
    while sleep "$POLL_INTERVAL"; do
        if wget -qO "$CA_TMP" "$HARVEST_CA_URL" 2>/dev/null; then
            if ! cmp -s "$CA_TMP" "$CA_FILE"; then
                echo "CA bundle changed, updating and reloading nginx..."
                mv "$CA_TMP" "$CA_FILE"
                nginx -s reload
            fi
        fi
    done
}
poll_and_reload &

exec /docker-entrypoint.sh "$@"
