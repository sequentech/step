// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot status that the check mark next to a Ballot ID shows, and the
// three steps its dialog lists, from a receipt's check. The worst step wins:
// a problem over a late step over one still on its way.

import type {ChainResult, TierName, TierStatus} from "@vitrine/verify"
import {EReceiptNotice, IReceiptView} from "./castReceipt"

export enum EBallotStatus {
    NOT_CAST = "notcast",
    CHECKING = "checking",
    RECORDED = "recorded",
    WITNESSED = "witnessed",
    VERIFIED = "verified",
    LATE = "late",
    PROBLEM = "problem",
}

/** Why a ballot shows a problem. */
export enum EBallotProblem {
    /** Validly signed objects contradict each other: evidence. */
    MISMATCH = "mismatch",
    /** An object didn't parse, verify or match: damaged data, not evidence. */
    DAMAGED = "damaged",
    /** The event requires a receipt, and there is none. */
    NO_RECEIPT = "no-receipt",
    /** Nothing could be checked: no verifier, no pinned policy, or no entry. */
    UNCHECKED = "unchecked",
}

/** The steps the dialog lists, in order. */
export enum EStatusStep {
    RECORD = "record",
    WITNESSES = "witnesses",
    ANCHORS = "anchors",
}

export enum EStepState {
    /** Before the ballot is cast: what the step will check. */
    UPCOMING = "upcoming",
    WAITING = "waiting",
    ACTIVE = "active",
    DONE = "done",
    LATE = "late",
    FAILED = "failed",
    /** Not checked, because an earlier step failed. */
    SKIPPED = "skipped",
}

export const STATUS_STEPS: readonly EStatusStep[] = [
    EStatusStep.RECORD,
    EStatusStep.WITNESSES,
    EStatusStep.ANCHORS,
]

const STEP_TIERS: Record<EStatusStep, readonly TierName[]> = {
    [EStatusStep.RECORD]: ["record"],
    [EStatusStep.WITNESSES]: ["witnesses"],
    [EStatusStep.ANCHORS]: ["hubs", "browser"],
}

/** A tier the result lacks has nothing to wait for. */
const tierStatus = (result: ChainResult, tier: TierName): TierStatus =>
    result.tiers.find((t) => t.tier === tier)?.status ?? "unused"

const isDone = (status: TierStatus): boolean => status === "verified" || status === "unused"

const isBad = (status: TierStatus): boolean => status === "failed" || status === "invalid"

/** The state of a step that rests on these tiers, on its own. */
export const tiersState = (result: ChainResult, tiers: readonly TierName[]): EStepState => {
    const statuses = tiers.map((tier) => tierStatus(result, tier))
    if (statuses.some(isBad)) return EStepState.FAILED
    if (statuses.some((status) => status === "late")) return EStepState.LATE
    return statuses.every(isDone) ? EStepState.DONE : EStepState.ACTIVE
}

const stepState = (result: ChainResult, step: EStatusStep): EStepState =>
    tiersState(result, STEP_TIERS[step])

export const ballotProblemOf = (view: IReceiptView | undefined): EBallotProblem | undefined => {
    if (view?.notice === EReceiptNotice.MISSING) return EBallotProblem.NO_RECEIPT
    if (view?.notice === EReceiptNotice.UNCHECKED) return EBallotProblem.UNCHECKED
    if (view?.notice === EReceiptNotice.WAITING) return undefined
    if (view?.result?.status === "failed") return EBallotProblem.MISMATCH
    if (view?.result?.status === "invalid") return EBallotProblem.DAMAGED
    return undefined
}

/** The status of a cast ballot, from its receipt's check; checking until
 * there is one. */
export const ballotStatusOf = (view: IReceiptView | undefined): EBallotStatus => {
    if (ballotProblemOf(view)) return EBallotStatus.PROBLEM
    const result = view?.notice === EReceiptNotice.NONE ? view.result : undefined
    if (!result) return EBallotStatus.CHECKING
    switch (result.status) {
        case "verified":
            return EBallotStatus.VERIFIED
        case "late":
            return EBallotStatus.LATE
        case "checking":
            return EBallotStatus.CHECKING
        default:
            if (stepState(result, EStatusStep.RECORD) !== EStepState.DONE) {
                return EBallotStatus.CHECKING
            }
            return stepState(result, EStatusStep.WITNESSES) === EStepState.DONE
                ? EBallotStatus.WITNESSED
                : EBallotStatus.RECORDED
    }
}

/** Each step's state. A step waits for the ones before it, and isn't checked
 * after one that failed. */
export const statusSteps = (
    status: EBallotStatus,
    result?: ChainResult
): Record<EStatusStep, EStepState> => {
    const own = (step: EStatusStep): EStepState => {
        if (status === EBallotStatus.NOT_CAST) return EStepState.UPCOMING
        if (result) return stepState(result, step)
        if (step !== EStatusStep.RECORD) return EStepState.WAITING
        return status === EBallotStatus.PROBLEM ? EStepState.FAILED : EStepState.ACTIVE
    }
    const states = {} as Record<EStatusStep, EStepState>
    let before: EStepState = EStepState.DONE
    for (const step of STATUS_STEPS) {
        const state = own(step)
        if (state === EStepState.UPCOMING || before === EStepState.DONE) {
            states[step] = state
        } else if (before === EStepState.FAILED || before === EStepState.SKIPPED) {
            states[step] = EStepState.SKIPPED
        } else {
            states[step] = EStepState.WAITING
        }
        before = states[step]
    }
    return states
}
