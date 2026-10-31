#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Builds a synthetic test PKI named like the PNPKI, a foreign commercial chain,
# and .p12 files in every encryption variant the browser signing module must
# open. Everything is fake: no real person, CA or key.
#
#   scripts/signing/make-test-p12.sh [OUT_DIR]
#
# OUT_DIR defaults to packages/admin-portal/src/lib/signing/__fixtures__. Only
# the committed files are written there: the .p12 files, the two chain PEMs,
# the two DER CRLs and fixtures.json. fixtures.json holds what the openssl CLI
# reports for each .p12 leaf (fingerprint, SPKI hash, names, validity, key
# usage); tests use it as an oracle that is independent of the code under test.
# Keys and the CA databases live in a temporary directory (WORK_DIR=... keeps
# them), so every run makes new keys; the names, serial numbers, validity dates
# and extensions are fixed.
# Needs OpenSSL >= 3.0 with the legacy provider (for RC2-40).
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$REPO/packages/admin-portal/src/lib/signing/__fixtures__}"
PASS="${P12_PASSWORD:-Demo-2028}"
UTF8_PASS="Señal-2028"
CYRILLIC_PASS="Пароль-2028"
CJK_PASS="密码-2028-🔐"
# pinned-latin1-unpad-utf8.p12 (password "Señal", one self-signed RSA leaf) is kept
# as found in review: deriving its AES key from the Latin-1 bytes of the password
# yields garbage that still passes the CBC padding check, which only a pinned salt
# and IV reproduce. The script lists it in fixtures.json but never rewrites it.
PINNED_PASS="Señal"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
case "$OUT" in
  / | "$HOME" | "$REPO" | "$REPO"/packages) echo "refusing to write fixtures into $OUT" >&2; exit 1 ;;
esac
if [[ -n "${WORK_DIR:-}" ]]; then
  # Only a new directory or one this script made earlier is emptied.
  if [[ -e "$WORK_DIR" && ! -e "$WORK_DIR/.make-test-p12-work" ]]; then
    echo "refusing to empty $WORK_DIR: not a make-test-p12 work directory" >&2; exit 1
  fi
  WORK="$WORK_DIR"; rm -rf "$WORK"; mkdir -p "$WORK"; touch "$WORK/.make-test-p12-work"
else
  WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT
fi
KEYS="$WORK/keys"
CA="$WORK/ca"
PEM="$WORK/pem"
mkdir -p "$KEYS" "$CA" "$PEM"
# pinned-*.p12 are kept: they can't be regenerated on demand (see below).
find "$OUT" -maxdepth 1 -type f \( -name '*.p12' ! -name 'pinned-*' -o -name '*.pem' -o -name '*.crl' -o -name fixtures.json \) -delete
# The config reads these at load time, so they need a value for every command.
export CA_DIR="$CA" CRL_URI=none

# Fixed validity windows (UTC).
CA_FROM=20250101000000Z;  CA_TO=20451231235959Z
OK_FROM=20260101000000Z;  OK_TO=20291231235959Z
OLD_FROM=20230101000000Z; OLD_TO=20250101000000Z     # expired
NEW_FROM=20350101000000Z; NEW_TO=20371231235959Z     # not yet valid
CRL_BASE="http://crl.test-pnpki.invalid"

cat >"$CA/openssl.cnf" <<'EOF'
[ ca ]
default_ca = CA_default

[ CA_default ]
dir             = $ENV::CA_DIR
database        = $dir/index.txt
new_certs_dir   = $dir/newcerts
serial          = $dir/serial
crlnumber       = $dir/crlnumber
default_md      = sha256
policy          = policy_any
preserve        = yes
unique_subject  = no
copy_extensions = none
email_in_dn     = no
default_crl_days = 3650

[ policy_any ]
countryName            = optional
organizationName       = optional
organizationalUnitName = optional
commonName             = supplied
serialNumber           = optional

[ req ]
distinguished_name = dn
prompt = no
[ dn ]
CN = placeholder

[ v3_root ]
basicConstraints       = critical, CA:TRUE
keyUsage               = critical, keyCertSign, cRLSign
subjectKeyIdentifier   = hash

[ v3_ica ]
basicConstraints       = critical, CA:TRUE, pathlen:0
keyUsage               = critical, keyCertSign, cRLSign
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always
crlDistributionPoints  = URI:$ENV::CRL_URI

[ v3_signer ]
basicConstraints       = critical, CA:FALSE
keyUsage               = critical, digitalSignature, nonRepudiation
extendedKeyUsage       = clientAuth, emailProtection
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always
crlDistributionPoints  = URI:$ENV::CRL_URI
certificatePolicies    = 1.3.6.1.4.1.99999.1.2.1

[ v3_signer_noku ]
basicConstraints       = critical, CA:FALSE
keyUsage               = critical, keyEncipherment
extendedKeyUsage       = clientAuth
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid:always
crlDistributionPoints  = URI:$ENV::CRL_URI
EOF

ossl() { openssl "$@" 2>>"$CA/openssl.log"; }

# key NAME rsa|ec : create (or reuse) KEYS/NAME.key
key() {
  local f="$KEYS/$1.key"
  [[ -s "$f" ]] && return 0
  case "$2" in
    rsa) ossl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$f" ;;
    rsa3072) ossl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:3072 -out "$f" ;;
    ec) ossl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 \
          -pkeyopt ec_param_enc:named_curve -out "$f" ;;
  esac
}

# init_ca DIR CRL_URI
init_ca() {
  mkdir -p "$CA/$1/newcerts"
  : >"$CA/$1/index.txt"
  echo "unique_subject = no" >"$CA/$1/index.txt.attr"
  echo 1000 >"$CA/$1/crlnumber"
}

# issue CA_DIR ISSUER_NAME(or -selfsign) KEY SUBJECT SERIAL EXT FROM TO OUT_PEM
issue() {
  local cadir="$1" issuer="$2" keyname="$3" subj="$4" serial="$5" ext="$6" from="$7" to="$8" out="$9"
  echo "$serial" >"$CA/$cadir/serial"
  ossl req -new -config "$CA/openssl.cnf" -key "$KEYS/$keyname.key" -subj "$subj" -out "$CA/tmp.csr"
  if [[ "$issuer" == "-selfsign" ]]; then
    CA_DIR="$CA/$cadir" CRL_URI="none" ossl ca -batch -notext -config "$CA/openssl.cnf" \
      -selfsign -keyfile "$KEYS/$keyname.key" -extensions "$ext" \
      -startdate "$from" -enddate "$to" -in "$CA/tmp.csr" -out "$out"
  else
    CA_DIR="$CA/$cadir" CRL_URI="$CRL_BASE/$cadir.crl" ossl ca -batch -notext -config "$CA/openssl.cnf" \
      -keyfile "$KEYS/$issuer.key" -cert "$PEM/$issuer.pem" -extensions "$ext" \
      -startdate "$from" -enddate "$to" -in "$CA/tmp.csr" -out "$out"
  fi
}

# p12 NAME KEY CERT CHAIN VARIANT FRIENDLY [PASSWORD]
p12() {
  local name="$1" keyname="$2" cert="$3" chain="$4" variant="$5" friendly="$6" pw="${7:-$PASS}"
  local common=(pkcs12 -export -inkey "$KEYS/$keyname.key" -in "$cert" -certfile "$chain"
                -name "$friendly" -passout "pass:$pw" -out "$OUT/$name.p12")
  case "$variant" in
    aes)  ossl "${common[@]}" -keypbe AES-256-CBC -certpbe AES-256-CBC -macalg sha256 -iter 2048 ;;
    3des) ossl "${common[@]}" -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 ;;
    rc2)  ossl "${common[@]}" -legacy ;;   # key: PBE-SHA1-3DES, certs: PBE-SHA1-RC2-40, MAC sha1
    rc2all) ossl "${common[@]}" -legacy -keypbe PBE-SHA1-RC2-40 -certpbe PBE-SHA1-RC2-40 ;;
    # One file, two schemes: PBES2 derives from the password's UTF-8 bytes, PKCS#12 PBE from its BMPString.
    mixrc2) ossl "${common[@]}" -legacy -keypbe AES-256-CBC -certpbe PBE-SHA1-RC2-40 ;;
    mix3des) ossl "${common[@]}" -keypbe AES-256-CBC -certpbe PBE-SHA1-3DES -macalg sha256 -iter 2048 ;;
    mix3deskey) ossl "${common[@]}" -keypbe PBE-SHA1-3DES -certpbe AES-256-CBC -macalg sha256 -iter 2048 ;;
  esac
}

# --- Test PNPKI: root -> Individual CA ---------------------------------------
init_ca pnpki-root; init_ca pnpki-individual-ca
key test-pnpki-root rsa3072
key test-pnpki-individual-ca rsa3072
issue pnpki-root -selfsign test-pnpki-root "/C=PH/O=Test PNPKI/CN=Test PNPKI Root CA" \
  01 v3_root "$CA_FROM" "$CA_TO" "$PEM/test-pnpki-root.pem"
issue pnpki-root test-pnpki-root test-pnpki-individual-ca \
  "/C=PH/O=Test PNPKI/CN=Test PNPKI Individual CA" 02 v3_ica "$CA_FROM" "$CA_TO" \
  "$PEM/test-pnpki-individual-ca.pem"
cat "$PEM/test-pnpki-individual-ca.pem" "$PEM/test-pnpki-root.pem" >"$PEM/test-pnpki-chain.pem"

# --- Example Commercial CA (foreign, not a staff issuer) ---------------------
init_ca commercial-root; init_ca commercial-ca
key example-commercial-root ec
key example-commercial-ca ec
issue commercial-root -selfsign example-commercial-root "/C=US/O=Example Commercial/CN=Example Commercial Root CA" \
  01 v3_root "$CA_FROM" "$CA_TO" "$PEM/example-commercial-root.pem"
issue commercial-root example-commercial-root example-commercial-ca \
  "/C=US/O=Example Commercial/CN=Example Commercial CA" 02 v3_ica "$CA_FROM" "$CA_TO" \
  "$PEM/example-commercial-ca.pem"
cat "$PEM/example-commercial-ca.pem" "$PEM/example-commercial-root.pem" >"$PEM/example-commercial-chain.pem"

ICA=test-pnpki-individual-ca
CHAIN="$PEM/test-pnpki-chain.pem"
person() { echo "/C=PH/O=Test PNPKI/OU=Individual/CN=$1/serialNumber=$2"; }

# signer NAME KEYTYPE CN IDNO SERIAL EXT FROM TO
signer() {
  key "$1" "$2"
  issue pnpki-individual-ca "$ICA" "$1" "$(person "$3" "$4")" "$5" "$6" "$7" "$8" "$PEM/$1.pem"
}

# Valid signers.
signer maria-santos-rsa rsa "MARIA L. SANTOS"  PH-0001 1001 v3_signer "$OK_FROM" "$OK_TO"
signer jose-reyes-ec    ec  "JOSE P. REYES"    PH-0002 1002 v3_signer "$OK_FROM" "$OK_TO"
signer ana-cruz-rsa     rsa "ANA M. CRUZ"      PH-0003 1003 v3_signer "$OK_FROM" "$OK_TO"
# Validity and key usage failures.
signer ramon-garcia-expired  rsa "RAMON T. GARCIA"   PH-0004 1004 v3_signer "$OLD_FROM" "$OLD_TO"
signer liza-aquino-notyet    ec  "LIZA B. AQUINO"    PH-0005 1005 v3_signer "$NEW_FROM" "$NEW_TO"
signer pedro-bautista-noku   rsa "PEDRO G. BAUTISTA" PH-0006 1006 v3_signer_noku "$OK_FROM" "$OK_TO"
signer carmen-villanueva-revoked rsa "CARMEN S. VILLANUEVA" PH-0007 1007 v3_signer "$OK_FROM" "$OK_TO"
# Reissue: a second certificate for MARIA's key pair (same SPKI, new serial).
issue pnpki-individual-ca "$ICA" maria-santos-rsa "$(person "MARIA L. SANTOS" PH-0001)" 1008 v3_signer \
  20260701000000Z "$OK_TO" "$PEM/maria-santos-rsa-reissue.pem"
# Same holder, two different keys (same subject DN).
signer juan-delacruz-a rsa "JUAN D. DELA CRUZ" PH-0009 1009 v3_signer "$OK_FROM" "$OK_TO"
signer juan-delacruz-b ec  "JUAN D. DELA CRUZ" PH-0009 1010 v3_signer "$OK_FROM" "$OK_TO"

# Foreign signer from the commercial chain.
key rosa-mendoza-foreign rsa
issue commercial-ca example-commercial-ca rosa-mendoza-foreign \
  "/C=PH/O=Example Commercial/CN=ROSA C. MENDOZA" 2001 v3_signer "$OK_FROM" "$OK_TO" \
  "$PEM/rosa-mendoza-foreign.pem"

# Revocation, then CRLs for both PNPKI levels (the root's is empty).
CA_DIR="$CA/pnpki-individual-ca" CRL_URI=none ossl ca -config "$CA/openssl.cnf" \
  -keyfile "$KEYS/$ICA.key" -cert "$PEM/$ICA.pem" \
  -revoke "$PEM/carmen-villanueva-revoked.pem" -crl_reason keyCompromise
CA_DIR="$CA/pnpki-individual-ca" CRL_URI=none ossl ca -config "$CA/openssl.cnf" -gencrl -crldays 3650 \
  -keyfile "$KEYS/$ICA.key" -cert "$PEM/$ICA.pem" -out "$PEM/test-pnpki-individual-ca.crl.pem"
CA_DIR="$CA/pnpki-root" CRL_URI=none ossl ca -config "$CA/openssl.cnf" -gencrl -crldays 3650 \
  -keyfile "$KEYS/test-pnpki-root.key" -cert "$PEM/test-pnpki-root.pem" -out "$PEM/test-pnpki-root.crl.pem"
for c in test-pnpki-individual-ca test-pnpki-root; do
  ossl crl -in "$PEM/$c.crl.pem" -outform DER -out "$OUT/$c.crl"
done

# --- PKCS#12 files ------------------------------------------------------------
p12 maria-santos-rsa-aes      maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" aes    "MARIA L. SANTOS"
p12 maria-santos-rsa-3des     maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" 3des   "MARIA L. SANTOS"
p12 maria-santos-rsa-rc2      maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" rc2    "MARIA L. SANTOS"
p12 maria-santos-rsa-rc2all   maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" rc2all "MARIA L. SANTOS"
p12 maria-santos-rsa-reissue  maria-santos-rsa "$PEM/maria-santos-rsa-reissue.pem" "$CHAIN" aes "MARIA L. SANTOS"
# Non-ASCII password: PBES2 derives from UTF-8 bytes, the MAC and PBE-SHA1 from BMPString.
p12 maria-santos-rsa-aes-utf8  maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" aes  "MARIA L. SANTOS" "$UTF8_PASS"
p12 maria-santos-rsa-3des-utf8 maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" 3des "MARIA L. SANTOS" "$UTF8_PASS"
p12 maria-santos-rsa-mixrc2-utf8 maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" mixrc2 "MARIA L. SANTOS" "$UTF8_PASS"
p12 maria-santos-rsa-mix3des-utf8 maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" mix3des "MARIA L. SANTOS" "$UTF8_PASS"
p12 maria-santos-rsa-mix3deskey-utf8 maria-santos-rsa "$PEM/maria-santos-rsa.pem" "$CHAIN" mix3deskey "MARIA L. SANTOS" "$UTF8_PASS"
# Multi-byte UTF-8 (2, 3 and 4 bytes per character) through PBKDF2.
p12 jose-reyes-ec-aes-cyrillic jose-reyes-ec "$PEM/jose-reyes-ec.pem" "$CHAIN" aes "JOSE P. REYES" "$CYRILLIC_PASS"
p12 ana-cruz-rsa-aes-cjk ana-cruz-rsa "$PEM/ana-cruz-rsa.pem" "$CHAIN" aes "ANA M. CRUZ" "$CJK_PASS"
p12 jose-reyes-ec-aes         jose-reyes-ec    "$PEM/jose-reyes-ec.pem"    "$CHAIN" aes    "JOSE P. REYES"
p12 jose-reyes-ec-3des        jose-reyes-ec    "$PEM/jose-reyes-ec.pem"    "$CHAIN" 3des   "JOSE P. REYES"
p12 ana-cruz-rsa-aes          ana-cruz-rsa     "$PEM/ana-cruz-rsa.pem"     "$CHAIN" aes    "ANA M. CRUZ"
for n in ramon-garcia-expired liza-aquino-notyet pedro-bautista-noku carmen-villanueva-revoked \
         juan-delacruz-a juan-delacruz-b; do
  p12 "$n" "$n" "$PEM/$n.pem" "$CHAIN" aes "$(openssl x509 -in "$PEM/$n.pem" -noout -subject -nameopt multiline | sed -n 's/^ *commonName *= //p')"
done
p12 rosa-mendoza-foreign rosa-mendoza-foreign "$PEM/rosa-mendoza-foreign.pem" \
  "$PEM/example-commercial-chain.pem" aes "ROSA C. MENDOZA"

# Certificates only: a .p12 without a private key.
ossl pkcs12 -export -nokeys -in "$PEM/maria-santos-rsa.pem" -certfile "$CHAIN" \
  -certpbe AES-256-CBC -macalg sha256 -iter 2048 -passout "pass:$PASS" -out "$OUT/no-key.p12"

rm -f "$CA/tmp.csr"
# Sanity: every valid PNPKI signer chains to the root.
openssl verify -CAfile "$PEM/test-pnpki-root.pem" -untrusted "$PEM/$ICA.pem" \
  "$PEM/maria-santos-rsa.pem" "$PEM/jose-reyes-ec.pem" "$PEM/maria-santos-rsa-reissue.pem" >/dev/null

cp "$PEM/test-pnpki-chain.pem" "$PEM/example-commercial-chain.pem" "$OUT/"

# --- fixtures.json: what openssl says about each leaf ------------------------
iso_date() { date -u -d "$1" +%Y-%m-%dT%H:%M:%SZ; }
leaf_json() { # FILE LEAF_PEM ALGORITHM PASSWORD CHAIN_LENGTH
  local file="$1" pem="$2" alg="$3" pw="$4" len="$5"
  local fp spki cn issuer_cn not_after ku signing
  fp=$(openssl x509 -in "$pem" -noout -fingerprint -sha256 | cut -d= -f2 | tr -d : | tr A-F a-f)
  spki=$(openssl x509 -in "$pem" -noout -pubkey | openssl pkey -pubin -outform DER | openssl dgst -sha256 -r | cut -d' ' -f1)
  cn=$(openssl x509 -in "$pem" -noout -subject -nameopt multiline | sed -n 's/^ *commonName *= //p')
  issuer_cn=$(openssl x509 -in "$pem" -noout -issuer -nameopt multiline | sed -n 's/^ *commonName *= //p')
  not_after=$(iso_date "$(openssl x509 -in "$pem" -noout -enddate | cut -d= -f2)")
  ku=$(openssl x509 -in "$pem" -noout -ext keyUsage | tail -n +2 | tr -d ' ')
  if [[ "$ku" == *DigitalSignature* || "$ku" == *NonRepudiation* ]]; then signing=true; else signing=false; fi
  printf '    "%s": {"password": "%s", "algorithm": "%s", "commonName": "%s", "issuerCommonName": "%s", "notAfter": "%s", "signingKeyUsage": %s, "chainLength": %s, "fingerprintSha256": "%s", "spkiSha256": "%s"}' \
    "$file" "$pw" "$alg" "$cn" "$issuer_cn" "$not_after" "$signing" "$len" "$fp" "$spki"
}
RSA=rsa-pkcs1-sha256
EC=ecdsa-p256-sha256
{
  echo "{"
  echo "  \"files\": {"
  entries=(
    "maria-santos-rsa-aes.p12|maria-santos-rsa|$RSA|$PASS|3"
    "maria-santos-rsa-3des.p12|maria-santos-rsa|$RSA|$PASS|3"
    "maria-santos-rsa-rc2.p12|maria-santos-rsa|$RSA|$PASS|3"
    "maria-santos-rsa-rc2all.p12|maria-santos-rsa|$RSA|$PASS|3"
    "maria-santos-rsa-aes-utf8.p12|maria-santos-rsa|$RSA|$UTF8_PASS|3"
    "maria-santos-rsa-3des-utf8.p12|maria-santos-rsa|$RSA|$UTF8_PASS|3"
    "maria-santos-rsa-mixrc2-utf8.p12|maria-santos-rsa|$RSA|$UTF8_PASS|3"
    "maria-santos-rsa-mix3des-utf8.p12|maria-santos-rsa|$RSA|$UTF8_PASS|3"
    "maria-santos-rsa-mix3deskey-utf8.p12|maria-santos-rsa|$RSA|$UTF8_PASS|3"
    "jose-reyes-ec-aes-cyrillic.p12|jose-reyes-ec|$EC|$CYRILLIC_PASS|3"
    "ana-cruz-rsa-aes-cjk.p12|ana-cruz-rsa|$RSA|$CJK_PASS|3"
    "maria-santos-rsa-reissue.p12|maria-santos-rsa-reissue|$RSA|$PASS|3"
    "jose-reyes-ec-aes.p12|jose-reyes-ec|$EC|$PASS|3"
    "jose-reyes-ec-3des.p12|jose-reyes-ec|$EC|$PASS|3"
    "ana-cruz-rsa-aes.p12|ana-cruz-rsa|$RSA|$PASS|3"
    "ramon-garcia-expired.p12|ramon-garcia-expired|$RSA|$PASS|3"
    "liza-aquino-notyet.p12|liza-aquino-notyet|$EC|$PASS|3"
    "pedro-bautista-noku.p12|pedro-bautista-noku|$RSA|$PASS|3"
    "carmen-villanueva-revoked.p12|carmen-villanueva-revoked|$RSA|$PASS|3"
    "juan-delacruz-a.p12|juan-delacruz-a|$RSA|$PASS|3"
    "juan-delacruz-b.p12|juan-delacruz-b|$EC|$PASS|3"
    "rosa-mendoza-foreign.p12|rosa-mendoza-foreign|$RSA|$PASS|3"
  )
  sep=""
  for e in "${entries[@]}"; do
    IFS='|' read -r file leaf alg pw len <<<"$e"
    printf '%s' "$sep"; leaf_json "$file" "$PEM/$leaf.pem" "$alg" "$pw" "$len"; sep=$',\n'
  done
  PINNED="$OUT/pinned-latin1-unpad-utf8.p12"
  if [[ -f "$PINNED" ]]; then
    openssl pkcs12 -in "$PINNED" -passin "pass:$PINNED_PASS" -nokeys -clcerts 2>/dev/null \
      | openssl x509 -out "$PEM/pinned-leaf.pem"
    printf '%s' "$sep"; leaf_json "pinned-latin1-unpad-utf8.p12" "$PEM/pinned-leaf.pem" "$RSA" "$PINNED_PASS" 1
  fi
  echo
  echo "  },"
  echo "  \"revokedSerials\": [\"$(openssl x509 -in "$PEM/carmen-villanueva-revoked.pem" -noout -serial | cut -d= -f2 | tr A-F a-f)\"]"
  echo "}"
} >"$OUT/fixtures.json"
echo "fixtures written to $OUT (password: $PASS)"
