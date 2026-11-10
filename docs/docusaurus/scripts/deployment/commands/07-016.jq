# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
include "board";
def client(id; f): .clients |= map(if .clientId == id then f else . end);
def drop(a; v): if a then a -= [v] else . end;
def portal(uris):
  .publicClient = true | .standardFlowEnabled = true
  | .implicitFlowEnabled = false | .directAccessGrantsEnabled = false
  | .redirectUris = uris | .webOrigins = ["+"]
  | .attributes["pkce.code.challenge.method"] = "S256"
  | .attributes["post.logout.redirect.uris"] = "+";
def machine(grant; account):
  .publicClient = false | .standardFlowEnabled = false | .implicitFlowEnabled = false
  | .directAccessGrantsEnabled = grant | .serviceAccountsEnabled = account
  | .redirectUris = [] | .webOrigins = [] | del(.authenticationFlowBindingOverrides)
  | .attributes["oauth2.device.authorization.grant.enabled"] = "false"
  | .attributes["oidc.ciba.grant.enabled"] = "false"
  | del(.attributes["post.logout.redirect.uris"]);
def execs(f): .authenticationFlows |= map(.authenticationExecutions |= map(f));
def password(v): [{type: "password", value: v, temporary: false}];
($ENV.PORTAL_HOST) as $p | ($ENV.ADMIN_HOST) as $a | ($ENV.VERIFIER_HOST) as $v
| .sslRequired = "external" | .ssoSessionIdleTimeout = 900
| .bruteForceProtected = true | .failureFactor = 5
| .eventsEnabled = true
| .eventsListeners = ((.eventsListeners // []) + ["custom-event-listener"] | unique)
| .rememberMe = false
# Keycloak registers idp_link, enabled, when it imports a document without it.
| .requiredActions |= ((if any(.[]; .alias == "idp_link") then . else . + [{alias: "idp_link",
    name: "Linking Identity Provider", providerId: "idp_link", priority: 100, config: {}}] end)
    | map(.enabled = false | .defaultAction = false))
| .identityProviders = ($idp | if $doc == "event-template" then .
    else map(.config.passMaxAge = "true") end) | .identityProviderMappers = []
| if $doc == "event-template" then . else
    client("admin-portal"; .protocolMappers =
      ([.protocolMappers[]? | select(.config["claim.name"] != "auth_time")]
      + [{name: "board-authentication-time", protocol: "openid-connect",
          protocolMapper: "oidc-usersessionmodel-note-mapper", consentRequired: false,
          config: {"user.session.note": "AUTH_TIME", "claim.name": "auth_time",
            "jsonType.label": "long", "id.token.claim": "true",
            "access.token.claim": "true", "introspection.token.claim": "true"}}])) end
| .authenticatorConfig = [(.authenticatorConfig // [])[]
    | select(.alias != "redirect-to-idp")]
    + [{alias: "redirect-to-idp", config: {defaultProvider: $idp[0].alias}}]
| execs(if .authenticator == "identity-provider-redirector"
          then .authenticatorConfig = "redirect-to-idp"
        elif .flowAlias == "sequent browser flow forms"
          or .flowAlias == "message channel OTP"
          then .requirement = "DISABLED"
        else . end)
| if any(.authenticationFlows[]; .alias == "certificate-first-login-flow") then .
  else .authenticationFlows += [$flow[0] | del(.id)] end
| client("admin-cli"; .directAccessGrantsEnabled = false)
| .authenticationFlows |= map(select(.alias != "smart-link" and .alias != "smart-link forms"))
| client("account"; .enabled = false) | client("account-console"; .enabled = false)
| client("security-admin-console"; .enabled = false)
| .defaultRole.name as $dr
| .roles.realm |= map(if .name == $dr
    then drop(.composites.client.account; "manage-account")
      | drop(.composites.realm; "offline_access") else . end)
| drop(.defaultOptionalClientScopes; "offline_access")
| .clients |= map(drop(.optionalClientScopes; "offline_access"))
| .components["org.keycloak.userprofile.UserProfileProvider"] |= map(
    .config["kc.user.profile.config"] |= map(fromjson
      | .attributes |= map(if .name == "username" or .name == "email"
          then drop(.permissions.edit; "user") else . end)
      | tojson))
| if $doc == "event-template" then
    client("voting-portal"; portal(["https://\($p)/*", "https://\($v)/*"]))
    | reduce ("results-portal", "onsite-voting-portal",
        "vp-sso", "ivr-service", "ivr-voting", "voting-portal-certs") as $c
        (.; client($c; .enabled = false))
    | del(.attributes["sequent.messaging"])
  else
    client("admin-portal"; portal(["https://\($a)/*"]))
    | client("voting-portal"; portal(["https://\($p)/*", "https://\($v)/*"]))
    | client("api-key-client"; machine(true; false)
        | .protocolMappers |= map(if .name == "x-hasura-default-role"
            then .config["claim.value"] = "trustee-ceremony" else . end))
    | client("service-account"; machine(false; true)
        | .authorizationServicesEnabled = false | del(.authorizationSettings))
    | if .users then .users |= map(select(.serviceAccountClientId != "api-key-client"))
      else . end
    | client("cli-account-admin"; .enabled = false | .directAccessGrantsEnabled = false)
    | .adminEventsEnabled = true | .adminEventsDetailsEnabled = true
    | .groups |= map(drop(.realmRoles; "publish-results-write")
        | drop(.realmRoles; "datafix-account")
        | drop(.realmRoles; "voter-manually-verify")
        | drop(.realmRoles; "offline_access")
        | if (.name | startswith("admin")) and .realmRoles
          then .realmRoles |= map(select(board_only | not)) else . end)
    | if $doc == "tenant" then
        client("service-account"; .secret = $ENV.KC_SERVICE_ACCOUNT_SECRET)
        | client("api-key-client"; .secret = $ENV.KC_API_KEY_CLIENT_SECRET)
        | .users |= map(
            if .username == "trustee1"
              then .credentials = password($ENV.TRUSTEE1_PASSWORD)
            elif .username == "trustee2"
              then .credentials = password($ENV.TRUSTEE2_PASSWORD)
            else del(.credentials) end)
      elif .users then .users |= map(del(.credentials))
      else . end
  end
