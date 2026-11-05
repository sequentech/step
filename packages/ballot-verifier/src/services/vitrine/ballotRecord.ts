// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What a ballot's record in Step's public ballot locator holds, as the voting
// portal's ballot tracker reads it (voting-portal
// services/vitrine/ballotTracker.ts): a cast ballot, which stays encrypted,
// or an audited one, with the plaintext and randomness of its ciphertext.

import type {IAuditableMultiBallot, IAuditableSingleBallot} from "@sequentech/ui-core"

/** What a ballot's record holds. */
export enum ERecordKind {
    /** A cast ballot, as the ballot box keeps it. */
    CAST = "cast",
    /** An audited ballot: with the plaintext and randomness of its ciphertext. */
    AUDITED = "audited",
    /** Neither: not JSON, or not a ballot. */
    UNREADABLE = "unreadable",
}

/** How an audited ballot holds its contests: a ciphertext for each, or one for all. */
export enum ERecordEncoding {
    SINGLE_CONTEST = "single-contest",
    MULTI_CONTEST = "multi-contest",
}

export type TRecord =
    | {kind: ERecordKind.CAST | ERecordKind.UNREADABLE}
    | {
          kind: ERecordKind.AUDITED
          encoding: ERecordEncoding.SINGLE_CONTEST
          ballot: IAuditableSingleBallot
      }
    | {
          kind: ERecordKind.AUDITED
          encoding: ERecordEncoding.MULTI_CONTEST
          ballot: IAuditableMultiBallot
      }

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const encodingOf = (contests: unknown): ERecordEncoding | undefined => {
    if (typeof contests === "string") return ERecordEncoding.MULTI_CONTEST
    return Array.isArray(contests) && contests.every((contest) => typeof contest === "string")
        ? ERecordEncoding.SINGLE_CONTEST
        : undefined
}

/** What a record's content is. A cast ballot names its ballot style by its
 * ID, in a string; an audited one carries the ballot style itself, and the
 * hash of the ballot, which the Tracking ID digests. */
export const classifyRecord = (content: string | null | undefined): TRecord => {
    const unreadable = {kind: ERecordKind.UNREADABLE} as const
    if (!content) return unreadable
    let record: unknown
    try {
        record = JSON.parse(content)
    } catch {
        return unreadable
    }
    if (!isObject(record)) return unreadable
    const encoding = encodingOf(record.contests)
    if (encoding === undefined) return unreadable
    if (typeof record.config === "string") return {kind: ERecordKind.CAST}
    if (!isObject(record.config) || typeof record.ballot_hash !== "string" || !record.ballot_hash) {
        return unreadable
    }
    return encoding === ERecordEncoding.SINGLE_CONTEST
        ? {kind: ERecordKind.AUDITED, encoding, ballot: record as unknown as IAuditableSingleBallot}
        : {kind: ERecordKind.AUDITED, encoding, ballot: record as unknown as IAuditableMultiBallot}
}
