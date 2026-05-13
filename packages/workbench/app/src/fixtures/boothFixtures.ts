// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Workbench fixtures for the booth lift.
//
// These are dispatched into voting-portal's PRODUCTION Redux store via its
// own action creators (`setElection`, `setElectionEvent`, `setBallotStyle`).
// The store shape, reducers and selectors are unchanged — only the data
// source is replaced. If the portal's slice types evolve, the fixtures
// below will fail TypeScript compilation, which is the early-warning
// signal we want.

import type {
    IBallotStyle as IBallotStyleEml,
    IContest,
    IElection,
} from "@sequentech/ui-core"
import {
    setElection,
    type IElectionExtended,
} from "voting-portal/src/store/elections/electionsSlice"
import {
    setElectionEvent,
    type IElectionEvent,
} from "voting-portal/src/store/electionEvents/electionEventsSlice"
import {
    setBallotStyle,
    type IBallotStyle,
} from "voting-portal/src/store/ballotStyles/ballotStylesSlice"
import {resetBallotSelection} from "voting-portal/src/store/ballotSelections/ballotSelectionsSlice"
import {store} from "voting-portal/src/store/store"

export const TENANT_ID = "00000000-0000-0000-0000-000000000001"
export const EVENT_ID = "00000000-0000-0000-0000-000000000002"
export const ELECTION_ID = "00000000-0000-0000-0000-000000000003"
export const BALLOT_STYLE_ID = "00000000-0000-0000-0000-0000000000b1"
export const CONTEST_ID = "00000000-0000-0000-0000-0000000000c1"
export const CANDIDATE_RED_ID = "00000000-0000-0000-0000-000000000a01"
export const CANDIDATE_BLUE_ID = "00000000-0000-0000-0000-000000000a02"

const contest: IContest = {
    id: CONTEST_ID,
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
            id: CANDIDATE_RED_ID,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: ELECTION_ID,
            contest_id: CONTEST_ID,
            name: "Red",
        },
        {
            id: CANDIDATE_BLUE_ID,
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: ELECTION_ID,
            contest_id: CONTEST_ID,
            name: "Blue",
        },
    ],
}

const election: IElectionExtended = {
    id: ELECTION_ID,
    election_event_id: EVENT_ID,
    tenant_id: TENANT_ID,
    name: "Workbench demo election",
    description: "A self-contained election fixture used by the booth lift.",
    image_document_id: "00000000-0000-0000-0000-0000000000ff",
    contests: [contest] satisfies IElection["contests"],
    // 0 = unlimited revotes, which keeps `canVoteSomeElection` true even
    // without a working castVotes feed.
    num_allowed_revotes: 0,
}

const electionEvent: IElectionEvent = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    name: "Workbench demo event",
    description: "Container event for the workbench fixture.",
    elections: [ELECTION_ID],
}

// `IBallotStyle.ballot_eml` is the ui-core ballot-style type, and is the
// shape VotingScreen consumes most of (`ballot_eml.contests`,
// `ballot_eml.public_key.is_demo`, `ballot_eml.election_event_presentation`,
// ...). Keep it in sync with the outer election fixture.
const ballotEml: IBallotStyleEml = {
    id: BALLOT_STYLE_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: ELECTION_ID,
    area_id: "00000000-0000-0000-0000-0000000000aa",
    contests: [contest],
    public_key: {
        // The default Ristretto election public key used by sequent-core's
        // own fixtures (`DEFAULT_PUBLIC_KEY_RISTRETTO_STR` in
        // sequent-core/src/encrypt.rs). It is a real point on the curve so
        // `encrypt_decoded_contest` succeeds; the matching private key
        // is intentionally not shipped — the workbench validates the
        // *encrypt* path, not the decrypt path.
        public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
        // `is_demo: false` so StartScreen doesn't pop the "this is a demo"
        // dialog; flip to true when exercising that flow.
        is_demo: false,
    },
}

const ballotStyle: IBallotStyle = {
    id: BALLOT_STYLE_ID,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: ELECTION_ID,
    area_id: "00000000-0000-0000-0000-0000000000aa",
    ballot_eml: ballotEml,
    created_at: new Date(0).toISOString(),
    last_updated_at: new Date(0).toISOString(),
}

let seeded = false

/**
 * Seed voting-portal's production Redux store with the booth fixture.
 * Idempotent: safe to call from a React effect that may re-fire.
 *
 * Why we also dispatch `resetBallotSelection` here: in production the
 * portal initializes the per-election `ballotSelections[electionId]`
 * entry from `StartScreen` (see `voting-portal/src/routes/StartScreen.tsx`)
 * when the voter clicks Start Voting. The `setBallotSelectionVoteChoice`
 * reducer is a no-op until that entry exists. The workbench needs every
 * URL to be a valid entry point (hot reload on `/vote`, deep links),
 * so we seed the empty-selection structure here too.
 */
export function seedBoothFixtures(): void {
    if (seeded) return
    seeded = true
    store.dispatch(setElection(election))
    store.dispatch(setElectionEvent(electionEvent))
    store.dispatch(setBallotStyle(ballotStyle))
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
}
