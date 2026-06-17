// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

// Fetches the public key registered for one trustee in one specific ceremony,
// from the per-ceremony trustee_ceremony_key table (keyed on
// trustee_id + election_event_id + keys_ceremony_id).
export const GET_TRUSTEE_CEREMONY_KEY = gql`
    query GetTrusteeCeremonyKey(
        $tenantId: uuid!
        $trusteeId: uuid!
        $electionEventId: uuid!
        $keysCeremonyId: uuid!
    ) {
        sequent_backend_trustee_ceremony_key(
            where: {
                tenant_id: {_eq: $tenantId}
                trustee_id: {_eq: $trusteeId}
                election_event_id: {_eq: $electionEventId}
                keys_ceremony_id: {_eq: $keysCeremonyId}
            }
        ) {
            public_key
        }
    }
`
