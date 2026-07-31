#!/usr/bin/env bash

# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

image_name="${1:-step-login-hint-keycloak-test}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
container_name="step-login-hint-test-${RANDOM}-$$"
test_dir="$(mktemp -d)"

cleanup() {
  docker rm -f "${container_name}" >/dev/null 2>&1 || true
  rm -rf "${test_dir}"
}
trap cleanup EXIT

docker run --detach \
  --name "${container_name}" \
  --publish 127.0.0.1::8080 \
  --volume "${script_dir}/login-hint-realm.json:/opt/keycloak/data/import/login-hint-test-realm.json:ro" \
  "${image_name}" \
  start-dev --http-enabled=true --hostname-strict=false --import-realm >/dev/null

port_mapping="$(docker port "${container_name}" 8080/tcp)"
keycloak_url="http://127.0.0.1:${port_mapping##*:}"
realm_url="${keycloak_url}/realms/login-hint-test"

for _attempt in $(seq 1 90); do
  if curl --silent --fail "${realm_url}/.well-known/openid-configuration" >/dev/null; then
    break
  fi
  if ! docker inspect --format '{{.State.Running}}' "${container_name}" | grep -qx true; then
    docker logs "${container_name}"
    exit 1
  fi
  sleep 1
done
curl --silent --fail "${realm_url}/.well-known/openid-configuration" >/dev/null

verifier="abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~"
challenge="$(python3 - "${verifier}" <<'PY'
import base64
import hashlib
import sys

print(base64.urlsafe_b64encode(hashlib.sha256(sys.argv[1].encode()).digest()).rstrip(b"=").decode())
PY
)"
oidc_query="client_id=voting-portal&redirect_uri=http%3A%2F%2F127.0.0.1%2Fcallback&response_type=code&scope=openid&state=state-value&nonce=nonce-value&code_challenge=${challenge}&code_challenge_method=S256"
redirect_oidc_query="${oidc_query/client_id=voting-portal/client_id=voting-portal-redirect}"
five_hints="login_hint__username=hint-user&login_hint__email=hint%40example.com&login_hint__firstName=Hint&login_hint__lastName=Voter&login_hint__reference=a%26b%3Dc"

assert_invalid_request() {
  local label="$1"
  local query="$2"
  local endpoint="${3:-auth}"
  local body_file="${test_dir}/${label}.body"
  local status
  status="$(curl --silent --show-error --output "${body_file}" --write-out '%{http_code}' \
    "${realm_url}/protocol/openid-connect/${endpoint}?${oidc_query}&${query}")"
  if [[ "${status}" != "400" ]]; then
    echo "${label}: expected HTTP 400, got ${status}" >&2
    exit 1
  fi
  python3 - "${body_file}" <<'PY'
import json
import pathlib
import sys

payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert payload == {
    "error": "invalid_request",
    "error_description": "Invalid login hint parameters",
}, payload
PY
}

assert_matrix_path_rejected() {
  local label="$1"
  local endpoint="$2"
  local body_file="${test_dir}/${label}.body"
  local status
  status="$(curl --silent --show-error --output "${body_file}" --write-out '%{http_code}' \
    "${realm_url}/protocol/openid-connect/${endpoint}?${oidc_query}&login_hint__username=private-sentinel&login_hint__username=other-private-sentinel")"
  if [[ "${status}" != "400" ]]; then
    echo "${label}: expected HTTP 400, got ${status}" >&2
    exit 1
  fi
  if grep -Fq 'private-sentinel' "${body_file}"; then
    echo "${label}: error response contains a login-hint value" >&2
    exit 1
  fi
}

assert_invalid_request \
  too-many \
  "login_hint__f0=v0&login_hint__f1=v1&login_hint__f2=v2&login_hint__f3=v3&login_hint__f4=v4&login_hint__f5=v5"
assert_invalid_request blank "login_hint__username="
assert_invalid_request valid-plus-blank \
  "login_hint__username=valid&login_hint__dateOfBirth="
assert_invalid_request duplicate \
  "login_hint__username=private-sentinel&login_hint__username=other-private-sentinel"
assert_invalid_request registration-duplicate \
  "login_hint__username=private-sentinel&login_hint__username=other-private-sentinel" \
  registrations
assert_matrix_path_rejected matrix-auth-duplicate 'auth;matrix=1'
assert_matrix_path_rejected matrix-registration-duplicate 'registrations;matrix=1'
assert_invalid_request oversized "login_hint__username=$(printf 'x%.0s' $(seq 1 256))"

# Quarkus rejects non-normalized encoded endpoint segments before JAX-RS dispatch. Keep that
# behavior covered so an encoded path cannot bypass the pre-matching filter.
encoded_path_index=0
for encoded_path in \
  "/realms/login-hint-test/protocol/openid-connect/%61uth" \
  "/%72ealms/login-hint-test/protocol/openid-connect/auth"; do
  encoded_path_body="${test_dir}/encoded-path-${encoded_path_index}.body"
  encoded_path_status="$(curl --silent --show-error --path-as-is \
    --output "${encoded_path_body}" --write-out '%{http_code}' \
    "${keycloak_url}${encoded_path}?${oidc_query}&login_hint__username=private-sentinel&login_hint__username=other-private-sentinel")"
  if [[ "${encoded_path_status}" != "400" ]]; then
    echo "encoded endpoint path: expected HTTP 400, got ${encoded_path_status}" >&2
    exit 1
  fi
  if grep -Fq 'private-sentinel' "${encoded_path_body}"; then
    echo "encoded endpoint error contains a login-hint value" >&2
    exit 1
  fi
  encoded_path_index=$((encoded_path_index + 1))
done

# The HTTP server may reject a malformed escape before JAX-RS runs. Either way it must be a visible
# 400, and no partial authorization request may be created.
python3 - "${keycloak_url##*:}" "${oidc_query}" <<'PY'
import socket
import sys

port = int(sys.argv[1])
path = (
    "/realms/login-hint-test/protocol/openid-connect/auth?"
    + sys.argv[2]
    + "&login_hint__username=%ZZ"
)
with socket.create_connection(("127.0.0.1", port), timeout=10) as connection:
    connection.sendall(
        (f"GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").encode()
    )
    response = b""
    while chunk := connection.recv(65536):
        response += chunk
status_line = response.split(b"\r\n", 1)[0]
assert status_line.startswith(b"HTTP/1.1 400") or status_line.startswith(b"HTTP/1.0 400"), status_line
assert b"%ZZ" not in response
PY

assert_prefilled_form() {
  local body_file="$1"
  python3 - "${body_file}" <<'PY'
from html.parser import HTMLParser
import pathlib
import sys

expected = {
    "username": "hint-user",
    "email": "hint@example.com",
    "firstName": "Hint",
    "lastName": "Voter",
    "reference": "a&b=c",
}

class Inputs(HTMLParser):
    def __init__(self):
        super().__init__()
        self.values = {}

    def handle_starttag(self, tag, attrs):
        if tag == "input":
            attributes = dict(attrs)
            if attributes.get("name") in expected:
                self.values[attributes["name"]] = attributes.get("value", "")

parser = Inputs()
parser.feed(pathlib.Path(sys.argv[1]).read_text())
assert parser.values == expected, parser.values
PY
}

direct_registration_body="${test_dir}/direct-registration.html"
direct_status="$(curl --silent --show-error --location \
  --output "${direct_registration_body}" --write-out '%{http_code}' \
  "${realm_url}/protocol/openid-connect/registrations?${oidc_query}&login_hint=hint-user&${five_hints}")"
[[ "${direct_status}" == "200" ]]
assert_prefilled_form "${direct_registration_body}"

excluded_fields_body="${test_dir}/excluded-fields.html"
excluded_fields_status="$(curl --silent --show-error --location \
  --output "${excluded_fields_body}" --write-out '%{http_code}' \
  "${realm_url}/protocol/openid-connect/registrations?${oidc_query}&login_hint__hiddenReference=excluded-private-sentinel&login_hint__pin=excluded-private-sentinel&login_hint__hiddenFlagReference=excluded-private-sentinel")"
[[ "${excluded_fields_status}" == "200" ]]
python3 - "${excluded_fields_body}" <<'PY'
from html.parser import HTMLParser
import pathlib
import sys

excluded = {"hiddenReference", "pin", "hiddenFlagReference"}

class Inputs(HTMLParser):
    def __init__(self):
        super().__init__()
        self.values = {}

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "input" and attributes.get("name") in excluded:
            self.values[attributes["name"]] = attributes.get("value", "")

parser = Inputs()
parser.feed(pathlib.Path(sys.argv[1]).read_text())
assert "excluded-private-sentinel" not in parser.values.values(), parser.values
PY

# Start at /auth with a client bound to RedirectToRegisterAuthenticator. Seeing all five hints on
# the resulting form proves that the authenticator retained the same authentication session and
# its client notes instead of reconstructing an incomplete registration request.
redirect_cookie_jar="${test_dir}/redirect-registration.cookies"
redirect_registration_body="${test_dir}/redirect-registration.html"
redirect_status="$(curl --silent --show-error --location \
  --cookie-jar "${redirect_cookie_jar}" --cookie "${redirect_cookie_jar}" \
  --output "${redirect_registration_body}" --write-out '%{http_code}' \
  "${realm_url}/protocol/openid-connect/auth?${redirect_oidc_query}&login_hint=hint-user&${five_hints}")"
[[ "${redirect_status}" == "200" ]]
assert_prefilled_form "${redirect_registration_body}"

# Complete a real authorization-code flow. A successful S256 exchange and the returned nonce prove
# that the filter leaves Keycloak-owned state, nonce and PKCE data untouched.
auth_cookie_jar="${test_dir}/auth.cookies"
auth_body="${test_dir}/auth.html"
curl --silent --show-error --location \
  --cookie-jar "${auth_cookie_jar}" --cookie "${auth_cookie_jar}" \
  --output "${auth_body}" \
  "${realm_url}/protocol/openid-connect/auth?${oidc_query}&login_hint=test-user&login_hint__username=test-user"
login_action="$(python3 - "${auth_body}" <<'PY'
from html.parser import HTMLParser
import pathlib
import sys

class LoginForm(HTMLParser):
    def __init__(self):
        super().__init__()
        self.action = None

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "form" and attributes.get("id") == "kc-form-login":
            self.action = attributes.get("action")

parser = LoginForm()
parser.feed(pathlib.Path(sys.argv[1]).read_text())
assert parser.action, "login form not found"
print(parser.action)
PY
)"
auth_headers="${test_dir}/auth.headers"
auth_status="$(curl --silent --show-error \
  --cookie-jar "${auth_cookie_jar}" --cookie "${auth_cookie_jar}" \
  --dump-header "${auth_headers}" --output "${test_dir}/auth-post.body" --write-out '%{http_code}' \
  --data-urlencode 'username=test-user' \
  --data-urlencode 'password=integration-test-password' \
  --data-urlencode 'credentialId=' \
  "${login_action}")"
[[ "${auth_status}" == "302" ]]
callback_location="$(python3 - "${auth_headers}" <<'PY'
import pathlib
import sys

for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    if line.lower().startswith("location:"):
        print(line.split(":", 1)[1].strip())
        break
else:
    raise AssertionError("authorization callback location not found")
PY
)"
authorization_code="$(python3 - "${callback_location}" <<'PY'
import sys
import urllib.parse

query = urllib.parse.parse_qs(urllib.parse.urlsplit(sys.argv[1]).query)
assert query["state"] == ["state-value"], query
print(query["code"][0])
PY
)"
token_body="${test_dir}/token.json"
curl --silent --show-error --fail --output "${token_body}" \
  --data-urlencode 'grant_type=authorization_code' \
  --data-urlencode 'client_id=voting-portal' \
  --data-urlencode 'redirect_uri=http://127.0.0.1/callback' \
  --data-urlencode "code=${authorization_code}" \
  --data-urlencode "code_verifier=${verifier}" \
  "${realm_url}/protocol/openid-connect/token"
python3 - "${token_body}" <<'PY'
import base64
import json
import pathlib
import sys

token = json.loads(pathlib.Path(sys.argv[1]).read_text())
payload = token["id_token"].split(".")[1]
payload += "=" * (-len(payload) % 4)
claims = json.loads(base64.urlsafe_b64decode(payload))
assert claims["nonce"] == "nonce-value", claims
PY

if docker logs "${container_name}" 2>&1 | grep -Fq 'private-sentinel'; then
  echo "Keycloak logs contain a login-hint value" >&2
  exit 1
fi

echo "Keycloak login-hint authorization endpoint integration tests passed"
