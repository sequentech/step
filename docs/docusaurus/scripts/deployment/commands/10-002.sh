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
for h in $PORTAL_HOST $ADMIN_HOST; do
  curl -s "https://$h/realms/$R/.well-known/openid-configuration" | jq -r .issuer
done
for h in $PORTAL_HOST $ADMIN_HOST $VERIFIER_HOST; do
  curl -s -o /dev/null -w "$h %{http_code}\n" \
    "https://$h/realms/master/.well-known/openid-configuration"
done
