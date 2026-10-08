// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {
    GetBallotBoxSealsQuery as GeneratedBallotBoxSealsQuery,
    GetBallotBoxSealsQueryVariables,
} from "@/gql/graphql"

export type {GetBallotBoxSealsQueryVariables}

/** The status of a seal row (the table's CHECK). */
export enum EBallotBoxSealStatus {
    /** Closed; sealed when the grace period ends. */
    PENDING = "pending",
    /** Sealed and locked; its entry is being posted to the bulletin board. */
    SEALED = "sealed",
    /** Sealed, on the bulletin board, with its public record. */
    PUBLISHED = "published",
    /** Not sealed: an incident. The ballot box stays locked. */
    FAILED = "failed",
}

/** How the election was closed (`closed_by.kind`). */
export enum EBallotBoxClosedByKind {
    USER = "user",
    SIGNED = "signed",
    SCHEDULED = "scheduled",
}

export interface IBallotBoxCloseSigner {
    name: string
    certificate_sha256?: string
}

export interface IBallotBoxClosedBy {
    kind: EBallotBoxClosedByKind
    username?: string | null
    signers?: Array<IBallotBoxCloseSigner> | null
    signing_code?: string | null
}

type GeneratedBallotBoxSeal =
    GeneratedBallotBoxSealsQuery["sequent_backend_ballot_box_seal"][number]

/**
 * The seal of one ballot box (VOTE-FREEZE), as `GetBallotBoxSeals` reads it
 * from `sequent_backend_ballot_box_seal`: the generated row, with what
 * codegen can't type (the status text check, the `closed_by` jsonb and the
 * bigint counts, which Hasura returns as numbers) made precise here.
 */
export type IBallotBoxSeal = Omit<
    GeneratedBallotBoxSeal,
    | "__typename"
    | "id"
    | "election_id"
    | "area_id"
    | "status"
    | "closed_by"
    | "ballots_in_box"
    | "ballots_counted"
> & {
    id: string
    election_id: string
    area_id: string
    status: EBallotBoxSealStatus
    closed_by: IBallotBoxClosedBy
    ballots_in_box?: number | null
    ballots_counted?: number | null
}

/** Why the sealer left a pending box pending on its last attempt (`waiting_reason`). */
export enum EBallotBoxWaitingReason {
    /** Its deadline hadn't passed. */
    DEADLINE = "deadline",
    /** A channel of the election is enabled and not closed (`channel_open:KIOSK`). */
    CHANNEL_OPEN = "channel_open",
    /** Datafix votes are in progress (`datafix_votes:3`). */
    DATAFIX_VOTES = "datafix_votes",
    /** Another run was sealing it. */
    BUSY = "busy",
    /** The attempt failed and is retried (`error:<short>`). */
    ERROR = "error",
}

export interface IBallotBoxWaiting {
    reason: EBallotBoxWaitingReason
    /** The code's detail: the channel, the number of votes or the error. */
    detail?: string
}

/** The waiting reason of a pending box, or null when there is none or it is unknown. */
export const parseWaitingReason = (value?: string | null): IBallotBoxWaiting | null => {
    if (!value) return null
    const separator = value.indexOf(":")
    const code = separator < 0 ? value : value.slice(0, separator)
    const detail = separator < 0 ? undefined : value.slice(separator + 1)
    const reason = Object.values(EBallotBoxWaitingReason).find((known) => known === code)
    return reason ? {reason, detail} : null
}

export type GetBallotBoxSealsQuery = Omit<
    GeneratedBallotBoxSealsQuery,
    "sequent_backend_ballot_box_seal"
> & {
    sequent_backend_ballot_box_seal: Array<IBallotBoxSeal>
}

/**
 * The ballot styles that tell an election's ballot boxes before the close,
 * the names of their areas, whether an event has seals and its failed seals:
 * the generated types.
 */
export type {
    GetBallotBoxAreaNamesQuery,
    GetBallotBoxAreaNamesQueryVariables,
    GetBallotBoxAreasQuery,
    GetBallotBoxAreasQueryVariables,
    GetEventBallotBoxSealsQuery,
    GetFailedBallotBoxSealsQuery,
} from "@/gql/graphql"
