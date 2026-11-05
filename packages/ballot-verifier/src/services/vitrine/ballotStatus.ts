// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot status that the receipt screen shows, and the three steps it
// lists, from the check's result. The worst step wins: a problem over a late
// step over one still on its way.

import type {BrowserLayer, ChainResult, Policy, TierName, TierStatus} from "@vitrine/verify"

export enum EBallotStatus {
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
}

/** The steps the result lists, in order. */
export enum EStatusStep {
    RECORD = "record",
    WITNESSES = "witnesses",
    ANCHORS = "anchors",
}

export enum EStepState {
    WAITING = "waiting",
    ACTIVE = "active",
    DONE = "done",
    LATE = "late",
    FAILED = "failed",
    DAMAGED = "damaged",
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

/** The reason of a record that isn't the entry the user looked for. The
 * library flags it `invalid`, but it is a mismatch, not damaged data. */
const ENTRY_MISMATCH = "entry-mismatch"

/** A tier's trouble, if it has any: a mismatch or damaged data. */
const tierTrouble = (
    result: ChainResult,
    tier: TierName
): EStepState.FAILED | EStepState.DAMAGED | undefined => {
    const found = result.tiers.find((t) => t.tier === tier)
    if (found?.status === "failed") return EStepState.FAILED
    if (found?.status !== "invalid") return undefined
    return found.reasons.some((r) => r.code === ENTRY_MISMATCH)
        ? EStepState.FAILED
        : EStepState.DAMAGED
}

/** The state of a step that rests on these tiers, on its own. */
export const tiersState = (result: ChainResult, tiers: readonly TierName[]): EStepState => {
    const trouble = tiers.map((tier) => tierTrouble(result, tier))
    if (trouble.includes(EStepState.FAILED)) return EStepState.FAILED
    if (trouble.includes(EStepState.DAMAGED)) return EStepState.DAMAGED
    const statuses = tiers.map((tier) => tierStatus(result, tier))
    if (statuses.includes("late")) return EStepState.LATE
    return statuses.every(isDone) ? EStepState.DONE : EStepState.ACTIVE
}

const stepState = (result: ChainResult, step: EStatusStep): EStepState =>
    tiersState(result, STEP_TIERS[step])

export const ballotProblemOf = (result: ChainResult | undefined): EBallotProblem | undefined => {
    if (!result) return undefined
    if (
        result.status === "failed" ||
        STATUS_STEPS.some((step) => stepState(result, step) === EStepState.FAILED)
    ) {
        return EBallotProblem.MISMATCH
    }
    return result.status === "invalid" ? EBallotProblem.DAMAGED : undefined
}

/** The status of a checked ballot; checking until there is a result. */
export const ballotStatusOf = (result: ChainResult | undefined): EBallotStatus => {
    if (!result) return EBallotStatus.CHECKING
    if (ballotProblemOf(result)) return EBallotStatus.PROBLEM
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
export const statusSteps = (result?: ChainResult): Record<EStatusStep, EStepState> => {
    const own = (step: EStatusStep): EStepState => {
        if (result) return stepState(result, step)
        return step === EStatusStep.RECORD ? EStepState.ACTIVE : EStepState.WAITING
    }
    const states = {} as Record<EStatusStep, EStepState>
    let before: EStepState = EStepState.DONE
    for (const step of STATUS_STEPS) {
        if (before === EStepState.DONE) {
            states[step] = own(step)
        } else if (
            before === EStepState.FAILED ||
            before === EStepState.DAMAGED ||
            before === EStepState.SKIPPED
        ) {
            states[step] = EStepState.SKIPPED
        } else {
            states[step] = EStepState.WAITING
        }
        before = states[step]
    }
    return states
}

/** What the steps say about this ballot's receipt, for their wording. */
export interface IStatusFacts {
    /** The entry's place in the record, once the record has it. */
    index?: number
    /** The witnesses that signed the record, those the policy names, and how
     * many of them its quorum needs, when each is known. */
    signed?: number
    witnesses?: number
    needed?: number
    /** The public hubs that hold the record. */
    hubs: number
    browser?: BrowserLayer
    /** When the anchors were due, for a late step: POSIX seconds. */
    anchorsDue?: number
    /** The steps that the policy doesn't use. */
    unused: readonly EStatusStep[]
}

/** The witnesses' quorum, when it is a plain k of the witnesses. */
const witnessQuorum = (policy: Policy): number | undefined => {
    const {witnesses, groups, quorum} = policy.witnesses
    const group = groups.get(quorum)
    return group && group.members.every((member) => witnesses.has(member)) ? group.k : undefined
}

const signedWitnesses = (result: ChainResult): number | undefined => {
    const quorum = result.steps.find((s) => s.code === "record-quorum" && s.ok === true)
    const signed = quorum?.params?.cosignatures
    return typeof signed === "number" && Number.isInteger(signed) && signed > 0 ? signed : undefined
}

export const statusFacts = (result: ChainResult, policy: Policy): IStatusFacts => {
    const witnesses = result.policy.witnesses.length
    const signed = signedWitnesses(result)
    const hubTier = result.tiers.find((t) => t.tier === "hubs")
    const named = (hubTier?.hubs ?? []).filter((hub) => !hub.test)
    const due = result.tiers
        .filter((t) => (t.tier === "hubs" || t.tier === "browser") && t.status === "late")
        .flatMap((t) => (t.deadline === undefined ? [] : [t.deadline]))
    return {
        index: result.entry?.index,
        ...(signed !== undefined && witnesses > 0 ? {signed: Math.min(signed, witnesses)} : {}),
        ...(witnesses > 0 ? {witnesses} : {}),
        needed: witnessQuorum(policy),
        hubs: named.filter((hub) => hub.status === "verified").length,
        browser: result.policy.browser,
        ...(due.length ? {anchorsDue: Math.min(...due)} : {}),
        unused: STATUS_STEPS.filter((step) =>
            STEP_TIERS[step].every((tier) => tierStatus(result, tier) === "unused")
        ),
    }
}
