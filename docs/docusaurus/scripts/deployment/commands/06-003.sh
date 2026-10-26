# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
ca=/root/deploy/auth-server-ca.pem
tls() { echo | openssl s_client -connect "$AUTH_HOST:443" -servername "$AUTH_HOST" \
  -CAfile "$ca" -verify_hostname "$AUTH_HOST" -verify_return_error "$@" 2>&1; }
tls -tls1_3 -ciphersuites TLS_AES_256_GCM_SHA384 -groups secp384r1 \
  | grep -E '^New,|Temp Key|public key is|Verify return'
tls -tls1_2 >/dev/null && echo "TLS 1.2 ACCEPTED" || echo "TLS 1.2 refused"
tls -tls1_3 -ciphersuites TLS_AES_128_GCM_SHA256 >/dev/null \
  && echo "AES-128 ACCEPTED" || echo "AES-128 refused"
tls -tls1_3 -groups X25519 >/dev/null && echo "X25519 ACCEPTED" || echo "X25519 refused"
