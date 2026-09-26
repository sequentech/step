// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The election event and keys ceremony as the keys ceremony widgets receive
// them, with the relationships their callers query left empty.
import {EElectionEventCeremoniesPolicy} from "@sequentech/ui-core"
import type {Sequent_Backend_Election_Event, Sequent_Backend_Keys_Ceremony} from "@/gql/graphql"
import {
    eventPresentation,
    eventRecord,
    keysCeremonyRecord,
    trusteeRecords,
    type StoryRecord,
} from "@/__stories__/fixtures"
import {IKeysCeremonyExecutionStatus, IKeysCeremonyTrusteeStatus} from "@/services/KeyCeremony"

export const KEYS_CEREMONY_RESOURCE = "sequent_backend_keys_ceremony"
/** Base64 of the synthetic encrypted private key the service hands out. */
export const PRIVATE_KEY = "c3ludGhldGljLWVuY3J5cHRlZC1rZXk="

export function ceremonyEvent(
    policy: EElectionEventCeremoniesPolicy = EElectionEventCeremoniesPolicy.MANUAL_CEREMONIES
): Sequent_Backend_Election_Event {
    return {
        ...eventRecord(),
        presentation: {...eventPresentation, ceremonies_policy: policy},
        elections: [],
        elections_aggregate: {nodes: []},
    }
}

/** A ceremony whose trustees have reached the given statuses, in trustee order. */
export function ceremony(
    execution_status: IKeysCeremonyExecutionStatus,
    trustees: IKeysCeremonyTrusteeStatus[],
    overrides: Partial<StoryRecord<Sequent_Backend_Keys_Ceremony>> = {}
): Sequent_Backend_Keys_Ceremony {
    const base = keysCeremonyRecord()
    const success = execution_status === IKeysCeremonyExecutionStatus.SUCCESS
    return {
        ...base,
        execution_status,
        status: {
            public_key: success ? "synthetic-public-key" : undefined,
            trustees: trustees.map((status, index) => ({name: trusteeRecords[index].name, status})),
            logs: [{created_date: "2026-01-05T09:00:00Z", log_text: "Created keys ceremony"}],
        },
        keys_ceremony_trustee_ids: [],
        keys_ceremony_trustee_ids_aggregate: {nodes: []},
        ...overrides,
    }
}
