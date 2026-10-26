# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
tls() { echo | openssl s_client -connect "$C1:443" -servername "$1" "${@:2}" 2>&1; }
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  echo "== $h"
  tls $h -tls1_3 -ciphersuites TLS_AES_256_GCM_SHA384 -groups secp384r1 \
    -verify_hostname $h -verify_return_error \
    | grep -E '^New,|Temp Key|public key is|Verify return'
  tls $h -tls1_2 >/dev/null && echo "TLS 1.2 ACCEPTED" || echo "TLS 1.2 refused"
  tls $h -tls1_3 -ciphersuites TLS_AES_128_GCM_SHA256 >/dev/null \
    && echo "AES-128 ACCEPTED" || echo "AES-128 refused"
  tls $h -tls1_3 -ciphersuites TLS_CHACHA20_POLY1305_SHA256 >/dev/null \
    && echo "ChaCha20 ACCEPTED" || echo "ChaCha20 refused"
  tls $h -tls1_3 -groups X25519 >/dev/null && echo "X25519 ACCEPTED" \
    || echo "X25519 refused"
done
tls other.invalid >/dev/null && echo "other name ACCEPTED" || echo "other name refused"
echo | openssl s_client -connect "$C1:443" -noservername >/dev/null 2>&1 \
  && echo "no-SNI ACCEPTED" || echo "no-SNI refused"
port() { timeout 5 bash -c "</dev/tcp/$1/$2" 2>/dev/null && echo "$1:$2 open" \
  || echo "$1:$2 closed"; }
port "$C1" 80
