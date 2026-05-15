// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shared building blocks for the three import flows
// (`SnapshotImport`, `PortalBallotStyleImport`, `VelvetElectionImport`).
//
// The flows all funnel into a single `PersistedSnapshot` and then
// `loadSnapshotViaReload(snap, null)`. This file keeps the bits each
// flow needs to synthesize portal slice rows from a partial input,
// re-key every ballot style with a fresh workbench keypair, and wire
// the eligibility overlay (`workbench.ballotStylePool` +
// `workbench.assignments`) so the active-voter swap immediately
// reflects voter ↔ ballot-style assignments.

import type {PersistedSnapshot} from "../persistence"
import type {WorkbenchExtraState, Voter} from "../workbenchStore"
import {generateKeypair} from "../tally"

/** Shape of one portal `ballotStyles` slice row, kept loose because
 *  this file does not (and must not) import voting-portal types.
 *  See `voting-portal/src/store/ballotStyles/ballotStylesSlice.ts`
 *  for the authoritative definition. */
export interface PortalBallotStyleRow {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    area_id?: string | null
    ballot_eml: {
        id?: string
        public_key?: {public_key: string; is_demo?: boolean} | null
        contests?: Array<{id: string} & Record<string, unknown>>
        [k: string]: unknown
    }
    created_at?: string
    last_updated_at?: string
    [k: string]: unknown
}

/** Minimal portal election slice row. */
export interface PortalElectionRow {
    id: string
    election_event_id: string
    tenant_id: string
    name: string
    description?: string
    contests: Array<{id: string} & Record<string, unknown>>
    num_allowed_revotes?: number
    status?: Record<string, unknown>
    [k: string]: unknown
}

/** Minimal portal election-event slice row. */
export interface PortalElectionEventRow {
    id: string
    tenant_id: string
    name: string
    description?: string
    elections: string[]
    status?: Record<string, unknown>
}

/** Standard status block used everywhere the booth checks
 *  voting_status. */
export const DEFAULT_OPEN_STATUS = {
    is_published: true,
    voting_status: "OPEN",
    kiosk_voting_status: "CLOSED",
    early_voting_status: "CLOSED",
    voting_period_dates: {},
    kiosk_voting_period_dates: {},
    early_voting_period_dates: {},
} as const

/** Generate a fresh keypair and stamp the pk into the row's
 *  `ballot_eml.public_key.public_key` so the encrypt path uses it.
 *  Returns the new keypair so the caller can install it under
 *  `workbench.keypairs[bsId]`. Mutates `row` in place. */
export async function rekeyBallotStyle(
    row: PortalBallotStyleRow
): Promise<{pkB64: string; skB64: string}> {
    const kp = await generateKeypair()
    if (!row.ballot_eml || typeof row.ballot_eml !== "object") {
        row.ballot_eml = {}
    }
    row.ballot_eml.public_key = {public_key: kp.pkB64, is_demo: false}
    return kp
}

/** Build a fresh voter persona. The id is generated identically to
 *  the one in `workbenchStore.addVoter`. */
export function makeVoter(displayName: string): Voter {
    const id =
        typeof crypto !== "undefined" && "randomUUID" in crypto
            ? crypto.randomUUID()
            : `voter-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
    return {id, displayName}
}

/** Build a `state.ballotSelections` entry for the given contests
 *  with every choice unselected. The booth's reducers expect a row
 *  to exist for each election before selections can be set, so we
 *  pre-seed one. */
export function emptyBallotSelection(
    contests: Array<{id: string; candidates?: unknown}>
): Array<Record<string, unknown>> {
    return contests.map((c) => {
        // The hand-edited default snapshot ships an explicit
        // `{id, selected: -1}` per candidate, so we mirror that
        // shape when the contest carries a candidate list. Without
        // it the booth's reducers can't translate clicks into
        // selection mutations.
        const candidates = Array.isArray(c.candidates) ? c.candidates : []
        const choices = candidates
            .filter(
                (cand): cand is {id: string} =>
                    !!cand &&
                    typeof (cand as {id?: unknown}).id === "string"
            )
            .map((cand) => ({id: cand.id, selected: -1}))
        return {
            contest_id: c.id,
            is_explicit_invalid: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices,
        }
    })
}

/** Build the assembled `PersistedSnapshot` from the pieces every
 *  importer produces. Sets `activeVoterId` to the first voter (if
 *  any) so the eligibility swap fires on the first render. */
export function assembleSnapshot(args: {
    electionEvent: PortalElectionEventRow
    election: PortalElectionRow
    /** All ballot styles available for this election (the pool). */
    ballotStyles: PortalBallotStyleRow[]
    /** Workbench keypairs, keyed by ballot-style id. */
    keypairs: Record<string, {pkB64: string; skB64: string}>
    /** Voter personas (already sorted however the importer wants). */
    voters: Voter[]
    /** Eligibility: voter id → ballot-style ids. */
    assignments: Record<string, string[]>
}): PersistedSnapshot {
    const {electionEvent, election, ballotStyles, keypairs, voters, assignments} =
        args
    const activeVoterId = voters[0]?.id ?? null
    // Initial slice entry: whichever BS is assigned to the active
    // voter (or the first one in the pool as a fallback so the booth
    // has *something* to render before any voter switch).
    let initialBs: PortalBallotStyleRow | undefined
    if (activeVoterId) {
        const assigned = assignments[activeVoterId] ?? []
        initialBs = ballotStyles.find((bs) => assigned.includes(bs.id))
    }
    if (!initialBs) initialBs = ballotStyles[0]
    const ballotStylesSlice: Record<string, PortalBallotStyleRow> = {}
    if (initialBs) ballotStylesSlice[election.id] = initialBs
    const ballotSelections: Record<string, unknown> = {}
    if (initialBs?.ballot_eml.contests) {
        ballotSelections[election.id] = emptyBallotSelection(
            initialBs.ballot_eml.contests
        )
    }
    const workbench: WorkbenchExtraState = {
        voters,
        activeVoterId,
        castBy: {},
        repairedCastVotes: {},
        keypairs,
        ballotStylePool: {[election.id]: ballotStyles},
        assignments,
    }
    return {
        version: "v1",
        // The whole `state` object is typed as RootState upstream;
        // we build a partial that the hydrator tolerates (it only
        // reads the slices it knows about). Cast at the seam.
        state: {
            elections: {[election.id]: election},
            electionEvent: {[electionEvent.id]: electionEvent},
            ballotStyles: ballotStylesSlice,
            ballotSelections,
            castVotes: {},
            extra: {bypassChooser: false, isVoted: {}},
            supportMaterials: {},
            documents: {},
            auditableBallots: {},
            confirmationScreenData: {},
        } as unknown as PersistedSnapshot["state"],
        workbench,
        parentId: null,
    }
}
