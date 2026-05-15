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

import {
    EEarlyVotingPolicy,
    EVotingStatus,
    type IBallotStyle as IBallotStyleEml,
    type IContest,
    ICountingAlgorithm,
    type IElection,
    type IElectionEventStatus,
    type IElectionStatus,
    type IPeriodDates,
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
import {setKeypair, type WorkbenchKeypair} from "../workbenchStore"

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
    // Counting algorithm is required by velvet-wasm's tally path
    // (`tally_plaintext_ballots` errors with "contest is missing
    // counting_algorithm" if absent). The portal's encrypt path is
    // lenient about it, but the workbench's inline tally is not — so
    // we set it explicitly. Plurality-at-large is the only method
    // sensible for a single-winner / max_votes=1 contest.
    counting_algorithm: ICountingAlgorithm.PLURALITY_AT_LARGE,
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

// Empty period-dates record used to satisfy the strict
// `IElectionStatus` / `IElectionEventStatus` shape. The chooser screen
// only reads `voting_status`, so the date strings are never inspected.
const emptyPeriodDates: IPeriodDates = {}

// `ElectionSelectionScreen`'s `ElectionWrapper` calls `isVotingOpen()`
// during render (`<SelectElection isOpen={isVotingOpen()} ...>`). For
// non-kiosk voters that resolves to:
//   (online OPEN && event online OPEN) || (early ON && event early OPEN)
// We set the online voting status OPEN on both election and event so the
// first conjunct short-circuits before the early-voting branch — which
// would otherwise dereference `ballot_eml.area_presentation` and crash
// if absent. We still set a benign `area_presentation` below for safety.
const electionStatus: IElectionStatus = {
    is_published: true,
    voting_status: EVotingStatus.OPEN,
    kiosk_voting_status: EVotingStatus.CLOSED,
    early_voting_status: EVotingStatus.CLOSED,
    voting_period_dates: emptyPeriodDates,
    kiosk_voting_period_dates: emptyPeriodDates,
    early_voting_period_dates: emptyPeriodDates,
}

const electionEventStatus: IElectionEventStatus = {
    is_published: true,
    voting_status: EVotingStatus.OPEN,
    kiosk_voting_status: EVotingStatus.CLOSED,
    early_voting_status: EVotingStatus.CLOSED,
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
    status: electionStatus,
}

const electionEvent: IElectionEvent = {
    id: EVENT_ID,
    tenant_id: TENANT_ID,
    name: "Workbench demo event",
    description: "Container event for the workbench fixture.",
    elections: [ELECTION_ID],
    status: electionEventStatus,
}

// `IBallotStyle.ballot_eml` is the ui-core ballot-style type, and is the
// shape VotingScreen consumes most of (`ballot_eml.contests`,
// `ballot_eml.public_key.is_demo`, `ballot_eml.election_event_presentation`,
// ...). Keep it in sync with the outer election fixture.
//
// Constructed lazily inside `seedBoothFixtures` (rather than at module
// load) so the caller can inject the workbench-owned public key. See
// `WorkbenchKeypair` in `workbenchStore.ts`.
function buildBallotStyle(publicKeyB64: string): IBallotStyle {
    const ballotEml: IBallotStyleEml = {
        id: BALLOT_STYLE_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        area_id: "00000000-0000-0000-0000-0000000000aa",
        contests: [contest],
        // `ElectionWrapper.isEarlyVotingPolicyEnabled()` reads
        // `ballot_eml.area_presentation.allow_early_voting` unconditionally.
        // It is currently short-circuited by the online-OPEN status set on the
        // election above, but seeding a NO_EARLY_VOTING area_presentation
        // makes the fixture robust against status changes.
        area_presentation: {
            allow_early_voting: EEarlyVotingPolicy.NO_EARLY_VOTING,
        },
        public_key: {
            // Public half of the workbench-owned Ristretto ElGamal
            // keypair (see `workbenchStore.WorkbenchKeypair`). Generated
            // on first boot and persisted; reused across reloads so
            // previously captured `castVote.content` ciphertexts stay
            // decryptable.
            //
            // We intentionally *do not* use sequent-core's in-tree
            // `DEFAULT_PUBLIC_KEY_RISTRETTO_STR` here: that constant
            // has no matching private key anywhere in the repo
            // (production uses threshold trustee keys), so reusing it
            // would cut off the encrypt -> decrypt -> tally loop the
            // workbench is built to exercise.
            public_key: publicKeyB64,
            // `is_demo: false` so StartScreen doesn't pop the "this is a demo"
            // dialog; flip to true when exercising that flow.
            is_demo: false,
        },
    }
    return {
        id: BALLOT_STYLE_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: ELECTION_ID,
        area_id: "00000000-0000-0000-0000-0000000000aa",
        ballot_eml: ballotEml,
        created_at: new Date(0).toISOString(),
        last_updated_at: new Date(0).toISOString(),
    }
}

let seeded = false

/**
 * Seed voting-portal's production Redux store with the booth fixture.
 * Idempotent: safe to call from a React effect that may re-fire.
 *
 * `keypair` is the workbench-owned ElGamal keypair for the seeded
 * ballot style. The public half becomes `ballot_eml.public_key` so
 * the portal's encrypt path encrypts cast ballots under a key we
 * own; the secret half is registered in the workbench overlay
 * (`workbenchStore.keypairs[ballotStyleId]`) so the decrypt bridge
 * can later recover plaintexts under the same scope. Keypair lives
 * per-ballot-style because that is the field name production uses
 * for the encryption key.
 *
 * Why we also dispatch `resetBallotSelection` here: in production the
 * portal initializes the per-election `ballotSelections[electionId]`
 * entry from `StartScreen` (see `voting-portal/src/routes/StartScreen.tsx`)
 * when the voter clicks Start Voting. The `setBallotSelectionVoteChoice`
 * reducer is a no-op until that entry exists. The workbench needs every
 * URL to be a valid entry point (hot reload on `/vote`, deep links),
 * so we seed the empty-selection structure here too.
 */
export function seedBoothFixtures(keypair: WorkbenchKeypair): void {
    if (seeded) return
    seeded = true
    const ballotStyle = buildBallotStyle(keypair.pkB64)
    store.dispatch(setElection(election))
    store.dispatch(setElectionEvent(electionEvent))
    store.dispatch(setBallotStyle(ballotStyle))
    store.dispatch(resetBallotSelection({ballotStyle, force: true}))
    setKeypair(ballotStyle.id, keypair)
}
