# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
. /root/deploy/hosts.env
cd /root/deploy && umask 077
read -r -s -p 'Client secret: ' IDP_SECRET && echo && export IDP_SECRET
jq -n --arg alias "$IDP_ALIAS" --arg issuer '<issuer>' \
  --arg auth '<authorization-endpoint>' --arg token '<token-endpoint>' \
  --arg jwks '<jwks-url>' --arg logout '<end-session-endpoint>' \
  --arg id '<client-id>' '{
  alias: $alias, displayName: $alias, providerId: "oidc", enabled: true,
  trustEmail: false, storeToken: false, linkOnly: false,
  firstBrokerLoginFlowAlias: "certificate-first-login-flow",
  config: {issuer: $issuer, authorizationUrl: $auth, tokenUrl: $token,
    jwksUrl: $jwks, logoutUrl: $logout, clientId: $id,
    clientSecret: $ENV.IDP_SECRET, clientAuthMethod: "client_secret_post",
    validateSignature: "true", useJwksUrl: "true",
    pkceEnabled: "true", pkceMethod: "S256"}}' > idp.json
unset IDP_SECRET
jq 'del(.config.clientSecret)' idp.json
