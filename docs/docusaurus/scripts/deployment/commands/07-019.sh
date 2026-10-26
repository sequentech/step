# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
for d in tenant tenant-template event-template; do
  jq -L /root/deploy -c 'include "board";
    {listener: (.eventsListeners | index("custom-event-listener") != null),
    results: [.groups[]?.realmRoles[]?, .users[]?.realmRoles[]?
      | select(. == "publish-results-write" or . == "datafix-account"
        or . == "voter-manually-verify")],
    admin_groups: [.groups[]? | select(.name | startswith("admin")) | .realmRoles[]?
      | select(board_only)],
    forms: [.authenticationFlows[].authenticationExecutions[]
      | select(.flowAlias == "sequent browser flow forms") | .requirement],
    redirect: [.authenticatorConfig[] | select(.alias == "redirect-to-idp")
      | .config.defaultProvider],
    broker_freshness: [.identityProviders[]
      | {alias, passMaxAge: (.config.passMaxAge // "false"),
         prompt: (.config.prompt // "")}],
    admin_auth_time: [.clients[] | select(.clientId == "admin-portal")
      | .protocolMappers[]? | select(.config["claim.name"] == "auth_time")
      | [.protocolMapper, .config["user.session.note"], .config["jsonType.label"],
         .config["id.token.claim"], .config["access.token.claim"]]],
    portals: [.clients[] | select(.clientId | test("^(admin|voting)-portal$"))
      | [.clientId, .directAccessGrantsEnabled,
         .attributes["pkce.code.challenge.method"]]],
    admin_cli: [.clients[] | select(.clientId == "admin-cli")
      | .directAccessGrantsEnabled],
    cli_account_admin: [.clients[] | select(.clientId == "cli-account-admin")
      | [.enabled, .directAccessGrantsEnabled]],
    smart_link: [.authenticationFlows[] | select((.alias | startswith("smart-link"))
      or any(.authenticationExecutions[]; .authenticator == "smart-link")) | .alias],
    admin_events: [.adminEventsEnabled, .adminEventsDetailsEnabled]}' realm-$d.new.json
done
jq -c '.defaultRole.name as $dr
  | {messaging: (.attributes["sequent.messaging"] // "absent"),
  code_step: [.authenticationFlows[].authenticationExecutions[]
    | select(.flowAlias == "message channel OTP") | .requirement],
  offline: [.defaultOptionalClientScopes[], .clients[].optionalClientScopes[]?,
    (.roles.realm[] | select(.name == $dr) | .composites.realm[]?)]
    | index("offline_access")}' realm-event-template.new.json
