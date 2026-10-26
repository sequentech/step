# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy
jq --arg u '<operator-user-name-at-the-authentication-server>' --arg m '<e-mail-address>' \
  '.users += [.users[] | select(.username == "admin")
    | {username: $u, email: $m, emailVerified, enabled: true, realmRoles, groups,
       attributes: {"tenant-id": .attributes["tenant-id"]}}]' \
  realm-tenant.new.json > realm-tenant.tmp && mv realm-tenant.tmp realm-tenant.new.json
jq -c '.users[] | select(.groups == ["/admin"]) | [.username, (.credentials != null)]' \
  realm-tenant.new.json
