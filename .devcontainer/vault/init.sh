#!/bin/sh
set -e
set -x

#vault login
vault auth enable approle
vault write auth/approle/role/harvest \
    secret_id_ttl=10m \
    token_num_uses=10 \
    token_ttl=20m \
    token_max_ttl=30m \
    secret_id_num_uses=0
vault read auth/approle/role/harvest/role-id
vault write -f auth/approle/role/harvest/secret-id
