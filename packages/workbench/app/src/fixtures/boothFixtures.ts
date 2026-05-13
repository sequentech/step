// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench fixtures for the booth lift.
//
// These are dispatched into voting-portal's PRODUCTION Redux store via its
// own action creators (`setElection`, `setElectionEvent`). The store
// shape, reducers and selectors are unchanged — only the data source is
// replaced. If the portal's slice types evolve, the fixtures below will
// fail TypeScript compilation, which is the early-warning signal we want.

import type {IElection} from "@sequentech/ui-core"
import {setElection, type IElectionExtended} from "voting-portal/src/store/elections/electionsSlice"
import {
    setElectionEvent,
    type IElectionEvent,
} from "voting-portal/src/store/electionEvents/electionEventsSlice"
import {store} from "voting-portal/src/store/store"

export const TENANT_ID = "00000000-0000-0000-0000-000000000001"
export const EVENT_ID = "00000000-0000-0000-0000-000000000002"
export const ELECTION_ID = "00000000-0000-0000-0000-000000000003"

const election: IElectionExtended = {
    id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    name: "Workbench demo election",
    description: "A self-contained election fixture used by the booth lift.",
    image_document_id: "00000000-0000-0000-0000-0000000000ff",
    contests: [
        {
            id: "00000000-0000-0000-0000-0000000000c1",
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: ELECTION_ID,
            name: "Favourite colour",
            description: "Pick exactly one option.",
            max_votes: 1,
            min_votes: 1,
            winning_candidates_num: 1,
            is_encrypted: true,
            candidates: [
                {
                    id: "00000000-0000-0000-0000-000000000a01",
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    election_id: ELECTION_ID,
                    contest_id: "00000000-0000-0000-0000-0000000000c1",
                    name: "Red",
                },
                {
                    id: "00000000-0000-0000-0000-000000000a02",
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    election_id: ELECTION_ID,
                    contest_id: "00000000-0000-0000-0000-0000000000c1",
                    name: "Blue",
                },
            ],
        },
    ] satisfies IElection["contests"],
}

const electionEvent: IElectionEvent = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    name: "Workbench demo event",
    description: "Container event for the workbench fixture.",
    elections: [ELECTION_ID],
}

let seeded = false

/**
 * Seed voting-portal's production Redux store with the booth fixture.
 * Idempotent: safe to call from a React effect that may re-fire.
 */
export function seedBoothFixtures(): void {
    if (seeded) return
    seeded = true
    store.dispatch(setElection(election))
    store.dispatch(setElectionEvent(electionEvent))
}
