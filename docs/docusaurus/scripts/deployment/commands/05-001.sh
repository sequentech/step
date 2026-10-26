# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
install -d -m 700 /root/deploy/tls && cd /root/deploy/tls
openssl ecparam -name secp384r1 -genkey -noout -out tls.key
chmod 600 tls.key
openssl req -new -key tls.key -subj "/CN=$PORTAL_HOST" -out tls.csr \
  -addext "subjectAltName=DNS:$PORTAL_HOST,DNS:$VERIFIER_HOST,DNS:$ADMIN_HOST"
openssl req -in tls.csr -noout -text | grep -E 'NIST CURVE|DNS:'
