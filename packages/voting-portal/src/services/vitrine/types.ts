// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The event's transparency mode, from `presentation.transparency`. */
export enum ETransparencyMode {
    DISABLED = "disabled",
    PUBLIC = "public",
    PRIVATE = "private",
}

/** What Harvest got from the ballots log at cast (`InsertCastVoteOutput.vitrine`). */
export enum EVitrineCastKind {
    RECEIPT = "receipt",
    PROMISE = "promise",
    QUEUED = "queued",
    UNAVAILABLE = "unavailable",
}

/** The names of an event's two logs in its policy. */
export enum EVitrineLog {
    BALLOTS = "ballots",
    EVENTS = "events",
}

/** The routes Harvest forwards to the sidecar for private events. */
export enum EVitrineForwardRoute {
    UPGRADE = "upgrade",
    OPENING = "opening",
    /** Authenticated event-wide discovery; body is an empty string. */
    POLICIES = "policies",
}

/** The package the config statement's `release` line names for this portal. */
export const VOTING_PORTAL_PACKAGE = "sequent/voting-portal"

/** `vitrine` of InsertCastVoteOutput: `text` is data for @vitrine/verify, never rendered. */
export interface IVitrineCast {
    kind: string
    text?: string | null
    reason?: string | null
}

export interface IVitrineLogLocation {
    url: string
    origin: string
}

/** `presentation.vitrine`: what voters read about the event's logs. */
export interface IVitrinePresentation {
    /** Lowercase hex of the policy ID, with or without `sha256:`. */
    policy_id?: string
    /** The sealed, signed policy note (public events only). */
    policy?: string
    logs?: {
        ballots?: IVitrineLogLocation
        events?: IVitrineLogLocation
    }
}

/** A signed statement and the proof that the events log holds it. */
export interface IVitrineLoggedStatement {
    /** The signed note text. */
    statement: string
    /** Base64 of the events-log entry. */
    entry: string
    /** The receipt chain whose receipt meets the quorum. */
    chain: string
}

/** `vitrine_voter_context`, as the portal reads it. */
export interface IVitrineVoterContext {
    mode: string
    policy?: string | null
    policy_ack?: string | null
    config?: IVitrineLoggedStatement | null
    opened?: IVitrineLoggedStatement | null
    style_hashes?: string[] | null
    release?: string | null
}
