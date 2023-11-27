#!/bin/sh
set -x
# Check if the data directory is empty
if [ ! -d /vault/sys ]; then
  # If it's empty, untar the pre-copied volume.tar.gz into it
  tar -xzf /opt/vault/volume.tar.gz -C /vault
fi

# Then call the original entrypoint command
vault server -config /opt/vault/config.hcl &
sleep 5
vault operator unseal $VAULT_KEY

# Now bring the vault server back into the foreground
# so that it becomes the main process of the container
wait