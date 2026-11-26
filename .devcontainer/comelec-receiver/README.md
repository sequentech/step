<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequent.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Portal-managed receiver authorization

Configure the reviewed Default tenant, the exact producer WebAuthn RP ID
and HTTPS origin, and the approval floor using the existing receiver
compose checker. The checker does not take a manager's user ID. Install
the matching Harvest, Windmill and Beat build together. Do not use this
profile to replace an existing site-pinned authority.

In the selected tenant, roster management is limited to signed Admin
Portal users who have all Configuration package permissions and are
currently enabled human members of the tenant's root `/admin` group.
The standard Keycloak directory must return the same enabled subject in
its non-service-account search; a missing or unverifiable identity fails
closed. Activation still requires fresh sign-in, candidate fingerprint confirmation,
the expected generation and the full roster checks. Roster management
does not grant election or package signer credentials.

The previous `SEQUENT_RECEIVER_ADMIN_SUBJECT` site file entry and
`WINDMILL_WEBAUTHN_PACKAGES_ROSTER_ADMIN_SUBJECTS` environment variable
are deprecated and ignored by the new version, including if left set at
upgrade. Before upgrading an existing receiver, review the recipient
tenant's `/admin` memberships: all eligible administrators can manage the
roster, not just those included in the previous per-person setting.
