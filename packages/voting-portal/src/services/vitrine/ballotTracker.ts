// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot tracker works out from the ID in its address, from the
// ballot's receipt and from its record: how the ID is looked up, what the
// record holds, and the four steps of the status block (design board
// TrackerResult).

import type {IAuditableMultiBallot, IAuditableSingleBallot} from "@sequentech/ui-core"
import type {ChainResult, Policy, TierName, TierResult} from "@vitrine/verify"
import {
    ballotIdChars,
    ballotIdInput,
    EBallotIdInput,
    FULL_BALLOT_ID_LENGTH,
    SHORT_BALLOT_ID_LENGTH,
    typedBallotId,
} from "./ballotId"
import {EBallotStatus, ballotStatusOf, EStepState, tiersState} from "./ballotStatus"
import {EReceiptNotice, IReceiptView} from "./castReceipt"

const ALPHABET = /^[0-9A-HJKMNP-TV-Z]+$/
const SECONDS = 60

export interface IBallotLookup {
    ballotId?: string
    ballotIdPrefix?: string
}

/** How the ID in the address is looked up: the start of an ID (8 characters
 * or more) by its prefix, a full one as it is. Nothing for what isn't a
 * Tracking ID: a voter pseudonym (`FTBE:MHRX`) among it. */
export const ballotLookupOf = (value: string | undefined): IBallotLookup | null => {
    const hex = ballotIdChars(value ?? "")
    if (!ALPHABET.test(hex) || hex.length < SHORT_BALLOT_ID_LENGTH) return null
    return hex.length < FULL_BALLOT_ID_LENGTH ? {ballotIdPrefix: hex} : {ballotId: hex}
}

/** What the search field holds: a short ID, the start of one, a full one, or
 * neither. */
export const trackerInput = (value: string): EBallotIdInput => ballotIdInput(value)

/** Whether the voter typed or pasted characters that can't be part of a
 * Tracking ID, or more than a full one. The field leaves them out. */
export const rejectedBallotId = (value: string): boolean =>
    ballotIdChars(value) !== typedBallotId(value).replace(/[\s-]/g, "")

/** The `accepted_at` of the log's entry for a cast vote, in POSIX seconds,
 * from the vote's `created_at`: the ballot box's time as RFC 3339, in
 * microseconds. */
export const acceptedAtOf = (createdAt: string | null | undefined): number | undefined => {
    if (!createdAt) return undefined
    const milliseconds = Date.parse(createdAt.replace(/\.\d+/, ""))
    return Number.isNaN(milliseconds) ? undefined : Math.floor(milliseconds / 1000)
}

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
 * hash of the ballot, which is the Tracking ID. */
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

/** A file name for the ballot's record, from the short Tracking ID:
 * `ballot-FTBEMHRX.json`. */
export const recordFileName = (ballotId: string): string => {
    const short = ballotIdChars(ballotId).slice(0, SHORT_BALLOT_ID_LENGTH)
    return `ballot${short ? `-${short}` : ""}.json`
}

export enum ETrackerStep {
    RECORD = "record",
    WITNESSES = "witnesses",
    HUBS = "hubs",
    BROWSER = "browser",
}

export const TRACKER_STEPS: readonly ETrackerStep[] = [
    ETrackerStep.RECORD,
    ETrackerStep.WITNESSES,
    ETrackerStep.HUBS,
    ETrackerStep.BROWSER,
]

export enum ETrackerState {
    DONE = "done",
    /** Being checked. */
    ACTIVE = "active",
    /** Due within the anchors' window: no need to wait for it. */
    SCHEDULED = "scheduled",
    WAITING = "waiting",
    LATE = "late",
    FAILED = "failed",
    SKIPPED = "skipped",
}

export interface IWitnessCount {
    signed: number
    total: number
    /** What the quorum asks, when it is a plain k of n. */
    needed?: number
}

export interface ITrackerStep {
    step: ETrackerStep
    state: ETrackerState
    /** POSIX seconds: when the step happened. */
    at?: number
    /** POSIX seconds: when a pending or late step was due. */
    due?: number
    /** Minutes until a scheduled step is due. */
    minutes?: number
    /** The record's entry in its log. */
    index?: number
    witnesses?: IWitnessCount
    hubs?: string[]
}

/** The check a view holds, unless it is damaged data rather than a result. */
export const trackedResult = (view: IReceiptView | undefined): ChainResult | undefined =>
    view?.notice === EReceiptNotice.NONE && view.result?.status !== "invalid"
        ? view.result
        : undefined

const tierOf = (result: ChainResult, step: ETrackerStep): TierResult | undefined =>
    result.tiers.find((tier) => tier.tier === (step as TierName))

const numberOf = (value: string | number | undefined): number | undefined =>
    typeof value === "number" ? value : undefined

/** The witnesses that signed the record, with the time their quorum was met. */
const quorumOf = (result: ChainResult): {signed?: number; at?: number} => {
    const params = result.steps.find(
        (step) => step.code === "record-quorum" && step.tier === "witnesses" && step.ok === true
    )?.params
    return {signed: numberOf(params?.cosignatures), at: numberOf(params?.quorumTime)}
}

const witnessCount = (policy: Policy | undefined, signed: number | undefined) => {
    const set = policy?.witnesses
    if (!set || signed === undefined) return undefined
    const root = set.groups.get(set.quorum)
    const plain = root?.members.every((member) => set.witnesses.has(member))
    const count: IWitnessCount = {signed, total: set.witnesses.size}
    if (root && plain) count.needed = root.k
    return count
}

const stateOf = (result: ChainResult | undefined, step: ETrackerStep): ETrackerState => {
    if (!result) {
        return step === ETrackerStep.RECORD ? ETrackerState.ACTIVE : ETrackerState.WAITING
    }
    switch (tiersState(result, [step as TierName])) {
        case EStepState.DONE:
            return ETrackerState.DONE
        case EStepState.LATE:
            return ETrackerState.LATE
        case EStepState.FAILED:
            return ETrackerState.FAILED
        default: {
            const anchor = step === ETrackerStep.HUBS || step === ETrackerStep.BROWSER
            return anchor && result.status !== "checking" && tierOf(result, step)?.deadline
                ? ETrackerState.SCHEDULED
                : ETrackerState.ACTIVE
        }
    }
}

/** The status block's steps, those the policy uses. A step waits for the ones
 * before it, and isn't checked after one that failed. */
export const trackerSteps = (view: IReceiptView | undefined): ITrackerStep[] => {
    const result = trackedResult(view)
    const quorum = result ? quorumOf(result) : {}
    let before = ETrackerState.DONE
    const steps: ITrackerStep[] = []
    for (const step of TRACKER_STEPS) {
        const tier = result && tierOf(result, step)
        if (tier?.status === "unused") continue
        let state = stateOf(result, step)
        if (before === ETrackerState.FAILED || before === ETrackerState.SKIPPED) {
            state = ETrackerState.SKIPPED
        } else if (before !== ETrackerState.DONE) {
            state = ETrackerState.WAITING
        }
        const item: ITrackerStep = {step, state}
        if (state === ETrackerState.DONE) {
            if (step === ETrackerStep.RECORD) {
                if (view?.own) item.at = view.own.acceptedAt
                if (result?.entry?.index !== undefined) item.index = result.entry.index
            }
            if (step === ETrackerStep.WITNESSES) {
                if (quorum.at !== undefined) item.at = quorum.at
                const witnesses = witnessCount(view?.policy, quorum.signed)
                if (witnesses) item.witnesses = witnesses
            }
        }
        if (tier?.deadline !== undefined && state !== ETrackerState.DONE) item.due = tier.deadline
        if (state === ETrackerState.SCHEDULED && item.due !== undefined && result) {
            item.minutes = Math.max(1, Math.ceil((item.due - result.now) / SECONDS))
        }
        if (step === ETrackerStep.HUBS && result) item.hubs = result.policy.hubs
        steps.push(item)
        before = state
    }
    return steps
}

/** How much of a ring is drawn: all of it for a step that has an outcome,
 * part of it for one being checked. */
export const RING_PROGRESS = {FULL: 1, CHECKING: 0.35, NONE: 0} as const

export const ringProgress = (state: ETrackerState | undefined): number => {
    switch (state) {
        case ETrackerState.DONE:
        case ETrackerState.LATE:
        case ETrackerState.FAILED:
            return RING_PROGRESS.FULL
        case ETrackerState.ACTIVE:
            return RING_PROGRESS.CHECKING
        default:
            return RING_PROGRESS.NONE
    }
}

/** What the status block shows for a ballot. */
export enum ETrackerView {
    /** The receipt's check, step by step. */
    STATUS = "status",
    /** Still looking for the receipt or checking it. */
    CHECKING = "checking",
    /** This device has no receipt for the ballot, and the log gave none. */
    NO_RECEIPT = "no-receipt",
    /** A receipt kept on this device that is still waiting for the log. */
    WAITING = "waiting",
    /** The receipt couldn't be checked: damaged data or no verifier. */
    UNCHECKED = "unchecked",
}

export interface ITrackerReceipt {
    view?: IReceiptView
    /** Whether this device keeps the ballot's receipt. */
    kept: boolean
    /** Whether the lookup has finished. */
    settled: boolean
}

export const trackerViewOf = ({view, kept, settled}: ITrackerReceipt): ETrackerView => {
    if (trackedResult(view)) return ETrackerView.STATUS
    if (!settled) return ETrackerView.CHECKING
    if (!kept) return ETrackerView.NO_RECEIPT
    if (view?.notice === EReceiptNotice.WAITING) return ETrackerView.WAITING
    return ETrackerView.UNCHECKED
}

/** The ballot's status, from a result that is shown, else checking. */
export const trackerStatusOf = (view: IReceiptView | undefined): EBallotStatus =>
    trackedResult(view) ? ballotStatusOf(view) : EBallotStatus.CHECKING
