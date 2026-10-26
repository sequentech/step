# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
read -r -p 'Realm to check [installation/customer] (installation): ' _agd_realm_choice
case "${_agd_realm_choice:-installation}" in
  installation) ;;
  customer)
    if [ -z "${CR:-}" ]; then
      printf 'Customer realm is not configured; complete customer tenant selection first.\n' >&2
      return 1
    fi
    R=$CR
    ;;
  *) printf 'Choose installation or customer; no check was run.\n' >&2; return 1 ;;
esac
printf 'Checking realm: %s\n' "$R"
A="https://$PORTAL_HOST/realms/$R/protocol/openid-connect"
q="client_id=admin-portal&response_type=code&scope=openid"
ru="redirect_uri=https%3A%2F%2F$ADMIN_HOST%2F"
pk="code_challenge=$(printf 'x%.0s' {1..43})&code_challenge_method=S256"
curl -s -o /dev/null -w '%{redirect_url}\n' "$A/auth?$q&$ru&$pk"
curl -s -o /dev/null -w '%{redirect_url}\n' "$A/auth?$q&$ru"
curl -s -o /dev/null -w '%{http_code}\n' \
  "$A/auth?$q&redirect_uri=https%3A%2F%2Fother.invalid%2F&$pk"
curl -s -d grant_type=password -d client_id=admin-portal -d username=x \
  -d password=x "$A/token" | jq -r .error
curl -s -d grant_type=password -d client_id=admin-cli -d username=x \
  -d password=x "$A/token" | jq -r .error
curl -s -d grant_type=password -d client_id=cli-account-admin -d username=x \
  -d password=x "$A/token" | jq -r .error
