# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
for d in tenant tenant-template event-template; do
  jq -c '{sslRequired, ssoSessionIdleTimeout, bruteForceProtected, failureFactor,
    eventsEnabled, registrationAllowed, resetPasswordAllowed, rememberMe,
    mfa_selector: [.requiredActions[] | select(.alias == "mfa-method-selector")
      | [.enabled, .defaultAction]],
    idps: [.identityProviders[].alias],
    users: [.users[]? | select(.credentials) | .username],
    disabled: [.clients[] | select(.enabled == false) | .clientId]}' \
    realm-$d.new.json
done
