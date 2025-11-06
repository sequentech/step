#!/bin/sh
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -x
# Check if the data directory is empty
if [ ! -d /vault/sys ]; then
  # If it's empty, untar the pre-copied volume.tar.gz into it
  tar -xzf /opt/vault/volume.tar.gz -C /vault
fi

# Then call the original entrypoint command
vault server -config /opt/vault/config.hcl &

# Loop indefinitely until the 'vault operator unseal' command succeeds
while true; do
    vault operator unseal "$VAULT_UNSEAL_KEY"
    RETVAL=$?

    # Check if the return code is 0 (success)
    if [ $RETVAL -eq 0 ]; then
        echo "Vault unsealed successfully."
        break # Exit the loop if the command succeeded
    else
        echo "Error unsealing Vault. Retrying in 1 second..."
        sleep 1 # Wait for 1 second before retrying
    fi
done
#sleep 5
vault secrets enable --version=1 --path=secrets kv

# Now bring the vault server back into the foreground
# so that it becomes the main process of the container
wait
