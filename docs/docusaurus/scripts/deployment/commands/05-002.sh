# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/tls
openssl x509 -in tls.crt -noout -subject -issuer -ext subjectAltName -enddate
openssl x509 -in tls.crt -noout -text | grep 'NIST CURVE'    # NIST CURVE: P-384
cmp <(openssl x509 -in tls.crt -noout -pubkey) <(openssl pkey -in tls.key -pubout) \
  && echo "certificate matches the key"
