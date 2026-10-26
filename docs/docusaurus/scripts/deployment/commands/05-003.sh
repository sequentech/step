# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
scp root@$C1:/root/deploy/tls/tls.crt .
openssl x509 -in tls.crt -out leaf.pem
awk '/BEGIN CERTIFICATE/{n++} n>1' tls.crt > chain.pem
chain_args=()
[ ! -s chain.pem ] || chain_args=(-untrusted chain.pem)
openssl verify -CAfile /etc/ssl/certs/ca-certificates.crt "${chain_args[@]}" leaf.pem
