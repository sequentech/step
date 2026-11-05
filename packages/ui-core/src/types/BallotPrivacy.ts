// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** How far a ballot got in the public log: its latest entry. */
export type BallotStage = "received" | "cast" | "audited"

/**
 * A ballot in the public log, once, at its latest stage. Own-ballot responses
 * use the same shape separately. The receipt key the ballot box signs is not
 * part of it: a ballot is named by its Tracking ID.
 */
export interface IBallotLogRow {
    /** The full Tracking ID; the screens show its first two groups. */
    tracking_id: string
    pseudonym: string
    stage: BallotStage
    /** When the ballot reached that stage. */
    last_step_at: string
    /** Empty until the ballot is cast. */
    voting_channel: string
    election_id: string
}

export interface IBallotLogPage {
    ballots: IBallotLogRow[]
    next_cursor: string | null
    total: number
    match_count: number
    match_count_capped: boolean
    /** With a search that looks at both, how many ballots match each way. */
    tracking_matches?: number | null
    pseudonym_matches?: number | null
}

export interface IForensicAddress {
    id: string
    ip: string
    country: string | null
    election_presentation?: Record<string, unknown> | null
    vote_count: number
    pseudonyms: string[]
    pseudonym_count?: number
    first_ballot: string | null
    last_ballot: string | null
    shared: boolean
}

export interface IForensicAddressDetail {
    ip: string
    country: string | null
    vote_count: number
    first_ballot: string | null
    last_ballot: string | null
    ballots: Array<{ballot_id: string | null; pseudonym: string; cast_at: string}>
    next_cursor: string | null
}

/**
 * A voter pseudonym as it is shown: `FTBE:MHRX`, with a colon, so it is never
 * taken for a Tracking ID (`FTBE-MHRX`). The stored form keeps its hyphen.
 */
export const pseudonymDisplay = (code: string): string =>
    /^[0-9A-Za-z]{4}-[0-9A-Za-z]{4}$/.test(code) ? `${code.slice(0, 4)}:${code.slice(5)}` : code
