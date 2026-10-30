// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot tracker works out from the ID in its address and from the
// ballot's receipt: how the ID is looked up, and the four steps of the
// status block (design board TrackerResult).

import type {ChainResult, Policy, TierName, TierResult} from "@vitrine/verify"
import {
    ballotIdHex,
    ballotIdInput,
    EBallotIdInput,
    SHORT_BALLOT_ID_LENGTH,
    typedBallotId,
} from "./ballotId"
import {EBallotStatus, ballotStatusOf, EStepState, tiersState} from "./ballotStatus"
import {EReceiptNotice, IReceiptView} from "./castReceipt"

/** Telephone voters read out the first four characters of their ballot ID. */
export const TELEPHONE_BALLOT_ID_LENGTH = 4
const HEX = /^[0-9a-f]+$/
const SECONDS = 60

export interface IBallotLookup {
    ballotId?: string
    ballotIdPrefix?: string
}

const isShortLength = (length: number, telephone: boolean): boolean =>
    length === SHORT_BALLOT_ID_LENGTH || (telephone && length === TELEPHONE_BALLOT_ID_LENGTH)

/** How the ID in the address is looked up: a short ID by its prefix, any
 * other ID as it is. Nothing for what isn't hex. */
export const ballotLookupOf = (
    value: string | undefined,
    telephone: boolean
): IBallotLookup | null => {
    const hex = ballotIdHex(value ?? "")
    if (!HEX.test(hex)) return null
    return isShortLength(hex.length, telephone) ? {ballotIdPrefix: hex} : {ballotId: hex}
}

/** What the search field holds: a short ID, a full one, or neither. Events
 * with telephone voting also take the first four characters. */
export const trackerInput = (value: string, telephone: boolean): EBallotIdInput => {
    const input = ballotIdInput(value)
    const hex = ballotIdHex(value)
    if (
        input === EBallotIdInput.INVALID &&
        telephone &&
        hex.length === TELEPHONE_BALLOT_ID_LENGTH &&
        HEX.test(hex)
    ) {
        return EBallotIdInput.SHORT
    }
    return input
}

/** Whether the voter typed or pasted characters that can't be part of a
 * Ballot ID, or more than a full one. The field leaves them out. */
export const rejectedBallotId = (value: string): boolean =>
    ballotIdHex(value) !== typedBallotId(value).replace(/ /g, "")

/** The `accepted_at` of the log's entry for a cast vote, in POSIX seconds,
 * from the vote's `created_at`: the ballot box's time as RFC 3339, in
 * microseconds. */
export const acceptedAtOf = (createdAt: string | null | undefined): number | undefined => {
    if (!createdAt) return undefined
    const milliseconds = Date.parse(createdAt.replace(/\.\d+/, ""))
    return Number.isNaN(milliseconds) ? undefined : Math.floor(milliseconds / 1000)
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
