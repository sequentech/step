// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Public ballot-store row. Own-ballot responses use the same shape separately. */
export interface IBallotLogRow {
    ballot_id: string
    pseudonym: string
    cast_at: string
    voting_channel: string
    election_id: string
}

export interface IBallotLogPage {
    ballots: IBallotLogRow[]
    next_cursor: string | null
    total: number
    match_count: number
    match_count_capped: boolean
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
