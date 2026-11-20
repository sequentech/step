// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The `authorized-election-ids` value that authorizes a voter for an election:
 * its alias (9.0's external ID), or its id when it has none. The Keycloak
 * mapper and windmill (`authorization_alias`) read the attribute this way.
 */
export const electionAuthorizationKey = (election: {id: string; alias?: string | null}) =>
    election.alias || election.id
