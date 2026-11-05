// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The checks the audited ballot screen runs on a ballot from the public
// record, the ones the voting portal's ballot tracker runs (voting-portal
// services/vitrine/ballotAudit.ts): the ballot's hash and voter key give the
// Tracking ID it was opened by, its ciphertext reproduces from the plaintext
// and randomness it carries, and only then does it decode into the voter's
// selections.

import {
    trackingId,
    verifyAuditableBallotCiphertext,
    verifyAuditableMultiBallotCiphertext,
} from "@sequentech/ui-core"
import type {
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    IDecodedVoteContest,
} from "@sequentech/ui-core"
import {IBallotService, provideBallotService} from "../BallotService"
import {ballotIdChars} from "./ballotId"
import {ERecordEncoding, ERecordKind, TRecord} from "./ballotRecord"

export enum EAuditOutcome {
    /** The ballot is the one the Tracking ID names, and its ciphertext reproduces. */
    VERIFIED = "verified",
    /** The ballot is not the one the Tracking ID names, or its ciphertext does not reproduce. */
    MISMATCH = "mismatch",
    /** The checks could not run: nothing that says anything about the ballot. */
    NOT_VERIFIABLE = "not-verifiable",
}

export type TAudit =
    | {outcome: EAuditOutcome.VERIFIED; selections: IDecodedVoteContest[]}
    | {outcome: EAuditOutcome.MISMATCH}
    | {outcome: EAuditOutcome.NOT_VERIFIABLE}

export type TAuditedRecord = Extract<TRecord, {kind: ERecordKind.AUDITED}>

export type TAuditService = Pick<
    IBallotService,
    | "hashBallot512"
    | "hashMultiBallot"
    | "decodeAuditableBallot"
    | "decodeAuditableMultiBallot"
    | "verifyAuditableBallotCiphertext"
    | "verifyAuditableMultiBallotCiphertext"
> & {
    trackingId: (ballotHash: string, voterSigningPk?: string | null) => string
}

export const provideAuditService = (): TAuditService => {
    const {hashBallot512, hashMultiBallot, decodeAuditableBallot, decodeAuditableMultiBallot} =
        provideBallotService()
    return {
        hashBallot512,
        hashMultiBallot,
        decodeAuditableBallot,
        decodeAuditableMultiBallot,
        verifyAuditableBallotCiphertext,
        verifyAuditableMultiBallotCiphertext,
        trackingId,
    }
}

const hashOf = (service: TAuditService, record: TAuditedRecord): string =>
    record.encoding === ERecordEncoding.MULTI_CONTEST
        ? service.hashMultiBallot(record.ballot)
        : service.hashBallot512(record.ballot)

const reproduces = (service: TAuditService, record: TAuditedRecord): boolean =>
    record.encoding === ERecordEncoding.MULTI_CONTEST
        ? service.verifyAuditableMultiBallotCiphertext(record.ballot)
        : service.verifyAuditableBallotCiphertext(record.ballot)

const decode = (service: TAuditService, record: TAuditedRecord): IDecodedVoteContest[] | null =>
    record.encoding === ERecordEncoding.MULTI_CONTEST
        ? service.decodeAuditableMultiBallot(record.ballot)
        : service.decodeAuditableBallot(record.ballot)

/** Checks an audited ballot against the Tracking ID it was opened by. Only a
 * ballot that passes both checks is decoded, so the selections of one that
 * does not are never at hand. A check that can't run -- a malformed ballot
 * style or public key, contests that don't fit the style -- is no evidence
 * against the ballot, and is told apart from one that fails. */
export const auditBallot = (
    service: TAuditService,
    record: TAuditedRecord,
    trackingId: string
): TAudit => {
    try {
        const expected = ballotIdChars(trackingId)
        const hash = hashOf(service, record)
        if (!expected || !hash) return {outcome: EAuditOutcome.NOT_VERIFIABLE}
        // The Tracking ID digests the ballot's hash with the key that signed
        // it; a ballot without a key digests an empty one.
        const computed = ballotIdChars(
            service.trackingId(hash, record.ballot.voter_signing_pk ?? null)
        )
        if (!computed) return {outcome: EAuditOutcome.NOT_VERIFIABLE}
        if (computed !== expected || !reproduces(service, record)) {
            return {outcome: EAuditOutcome.MISMATCH}
        }
        const selections = decode(service, record)
        return selections
            ? {outcome: EAuditOutcome.VERIFIED, selections}
            : {outcome: EAuditOutcome.NOT_VERIFIABLE}
    } catch (error) {
        console.error("Failed to check the audited ballot:", error)
        return {outcome: EAuditOutcome.NOT_VERIFIABLE}
    }
}
