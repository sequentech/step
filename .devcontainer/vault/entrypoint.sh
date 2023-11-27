#!/bin/sh
# Check if the data directory is empty
if [ -z "$(ls -A /vault)" ]; then
  # If it's empty, untar the pre-copied volume.tar.gz into it
  tar -xzf /opt/vault/volume.tar.gz -C /vault
fi

# Then call the original entrypoint command
exec vault server -config /opt/vault/config.hcl