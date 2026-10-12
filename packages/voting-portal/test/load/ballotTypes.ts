// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** The cast operation carried between diagnostic preparation and transport workers. */
export interface CastPayload {
    operationName: string
    query: string
    variables: {electionId: string; ballotId: string; content: string}
}

/** A browser-observed request may not yet have its authorization header available. */
export interface CapturedCast {
    url: string
    payload: CastPayload
    authorization: string | null
}

/** Private prepared data; never embed this structure in aggregate reports. */
export interface PreparedBallot extends CapturedCast {
    authorization: string
    credentials: Record<string, string>
    tenant_id: string
    election_event_id: string
    style_id: string
    publication_version: string
    token_expires_at: number
    prepared_at: string
}

/** Minimal identity needed to independently match an accepted receipt. */
export interface CastReceipt {
    id: string
    ballot_id: string
    tenant_id: string
    election_id: string
    election_event_id: string
}

/** Publication references are scoped to the authenticated voter's election event. */
export interface VoterStatus {
    event_id: string
    files: {id: string; version: string; election_id: string; urls: Record<string, string>}[]
}
