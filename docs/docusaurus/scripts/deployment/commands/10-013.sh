# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
E=<name-of-the-election-event-realm>
curl -s "https://$PORTAL_HOST/realms/$E/messaging-channels" | jq -c .code_channels
curl -s "https://$PORTAL_HOST/realms/$E/.well-known/openid-configuration" \
  | jq -r .issuer
curl -s -d grant_type=password -d client_id=admin-cli -d username=x \
  -d password=x "https://$PORTAL_HOST/realms/$E/protocol/openid-connect/token" \
  | jq -r .error
