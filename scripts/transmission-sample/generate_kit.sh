#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Generates the synthetic keys and certificates of the ER transmission sample
# kit: the event's root and intermediate CAs, three SBEI certificates (.p12),
# and the key pair of the "CCS Manila (SBOC)" destination. Every key here is
# for testing only. Rerunning it replaces the kit, and the destination's
# public key and the CAs in transmission-sample-config.json must be updated
# with it (load_sample_config.py --print-kit-values prints them).
set -euo pipefail

KIT="$(cd "$(dirname "$0")" && pwd)/kit"
DAYS=7300
P12_PASSWORD="${P12_PASSWORD:-sample}"
mkdir -p "$KIT"
cd "$KIT"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

openssl ecparam -name prime256v1 -genkey -noout -out "$WORK/root.key"
openssl req -x509 -new -key "$WORK/root.key" -days "$DAYS" -sha256 \
    -subj "/C=PH/O=Sample Commission on Elections/CN=Sample OV Root CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -out root-ca.pem

openssl ecparam -name prime256v1 -genkey -noout -out "$WORK/intermediate.key"
openssl req -new -key "$WORK/intermediate.key" \
    -subj "/C=PH/O=Sample Commission on Elections/CN=Sample OV SBEI CA" \
    -out "$WORK/intermediate.csr"
printf 'basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n' \
    > "$WORK/intermediate.ext"
openssl x509 -req -in "$WORK/intermediate.csr" -CA root-ca.pem -CAkey "$WORK/root.key" \
    -CAcreateserial -days "$DAYS" -sha256 -extfile "$WORK/intermediate.ext" \
    -out intermediate-ca.pem

printf 'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,nonRepudiation\n' \
    > "$WORK/sbei.ext"

# The first and third SBEIs sign with RSA keys, the second with an EC key:
# the package takes both.
sbei() {
    local name="$1" common_name="$2" kind="$3"
    if [[ "$kind" == rsa ]]; then
        openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$WORK/$name.key"
    else
        openssl ecparam -name prime256v1 -genkey -noout -out "$WORK/$name.key"
    fi
    openssl req -new -key "$WORK/$name.key" \
        -subj "/C=PH/O=Sample Commission on Elections/CN=$common_name" \
        -out "$WORK/$name.csr"
    openssl x509 -req -in "$WORK/$name.csr" -CA intermediate-ca.pem \
        -CAkey "$WORK/intermediate.key" -CAcreateserial -days "$DAYS" -sha256 \
        -extfile "$WORK/sbei.ext" -out "$name.pem"
    openssl pkcs12 -export -inkey "$WORK/$name.key" -in "$name.pem" \
        -certfile intermediate-ca.pem -name "$common_name" \
        -keypbe AES-256-CBC -certpbe AES-256-CBC -macalg sha256 \
        -passout "pass:$P12_PASSWORD" -out "$name.p12"
}
sbei sbei-madrid-1 "Maria L. Santos" rsa
sbei sbei-madrid-2 "Jose R. Dela Cruz" ec
sbei sbei-madrid-3 "Ana P. Reyes" rsa
cat sbei-madrid-1.pem sbei-madrid-2.pem sbei-madrid-3.pem > sbei-certificates.pem
rm -f sbei-madrid-1.pem sbei-madrid-2.pem sbei-madrid-3.pem

# The destination's key pair: the package is sealed for its public key, and
# only the receiving side has the private key.
openssl ecparam -name prime256v1 -genkey -noout -out "$WORK/destination.key"
openssl pkcs8 -topk8 -nocrypt -in "$WORK/destination.key" -out ccs-manila-private-key.pem
openssl ec -in "$WORK/destination.key" -pubout -out ccs-manila-public-key.pem

# Another destination's key, for the altered-copy check that a package does
# not open with someone else's key.
openssl ecparam -name prime256v1 -genkey -noout -out "$WORK/other.key"
openssl pkcs8 -topk8 -nocrypt -in "$WORK/other.key" -out other-destination-private-key.pem

rm -f ./*.srl
echo "Kit written to $KIT"
