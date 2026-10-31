// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot status dialog says (design board ReceiptDialog): the
// headline, the three steps with the receipt's own facts, and the caption.
// Each text is an i18n key with its values, so that the words stay in the
// translations and what they claim stays here, next to the data it rests on.

import type {ChainResult, Policy} from "@vitrine/verify"
import {
    EBallotProblem,
    EBallotStatus,
    EStatusStep,
    EStepState,
    STATUS_STEPS,
    STEP_TIERS,
    statusSteps,
} from "./ballotStatus"

const STATUS_KEYS = "vitrine.status"

/** How long the browser layer takes when the policy doesn't say. */
const DEFAULT_ANCHOR_MINUTES = 10

const SECONDS_PER_MINUTE = 60

export interface ITranslation {
    key: string
    values?: Record<string, string | number>
}

export interface IStatusRow {
    step: EStatusStep
    state: EStepState
    title: ITranslation
    detail: ITranslation
    /** The word at the end of the row. */
    status?: ITranslation
    /** When this page saw the step done, in milliseconds. */
    doneAt?: number
}

export interface IStatusInput {
    status: EBallotStatus
    problem?: EBallotProblem
    result?: ChainResult
    policy?: Policy
    /** When this page saw each step done, in milliseconds. */
    times?: Partial<Record<EStatusStep, number>>
}

const pad = (n: number): string => String(n).padStart(2, "0")

/** A moment in POSIX seconds, as the hour and minute on this device's clock. */
export const clockMinutes = (seconds: number): string => {
    const at = new Date(seconds * 1000)
    return `${pad(at.getHours())}:${pad(at.getMinutes())}`
}

/** A moment in milliseconds, as the time on this device's clock. */
export const clockSeconds = (milliseconds: number): string => {
    const at = new Date(milliseconds)
    return `${pad(at.getHours())}:${pad(at.getMinutes())}:${pad(at.getSeconds())}`
}

const text = (key: string, values?: ITranslation["values"]): ITranslation => ({
    key: `${STATUS_KEYS}.${key}`,
    ...(values ? {values} : {}),
})

/** The minutes the browser layer should take, from the policy's period. */
export const anchorMinutes = (policy: Policy | undefined): number =>
    policy
        ? Math.max(1, Math.ceil(policy.browserPeriod / SECONDS_PER_MINUTE))
        : DEFAULT_ANCHOR_MINUTES

const stepOfLate = (steps: Record<EStatusStep, EStepState>): EStatusStep | undefined =>
    STATUS_STEPS.find((step) => steps[step] === EStepState.LATE)

export interface IStatusHeadline {
    head: ITranslation
    sub: ITranslation
}

/** What the dialog's headline says. The problem is why a ballot shows one. */
export const statusHeadline = (input: IStatusInput): IStatusHeadline => {
    const {status, problem, result, policy} = input
    const index = result?.entry?.index
    switch (status) {
        case EBallotStatus.WITNESSED:
            return {
                head: text("state.witnessed"),
                sub: text("sub.witnessed", {minutes: anchorMinutes(policy)}),
            }
        case EBallotStatus.LATE: {
            const late = stepOfLate(statusSteps(status, result))
            return {
                head: text("head.late"),
                sub: text(late === EStatusStep.ANCHORS ? "sub.late" : "sub.lateStep"),
            }
        }
        case EBallotStatus.PROBLEM:
            switch (problem) {
                case EBallotProblem.DAMAGED:
                    return {head: text("head.damaged"), sub: text("sub.damaged")}
                case EBallotProblem.NO_RECEIPT:
                    return {head: text("head.noReceipt"), sub: text("sub.noReceipt")}
                case EBallotProblem.UNCHECKED:
                    return {head: text("head.unchecked"), sub: text("sub.unchecked")}
                default:
                    return {
                        head: text("head.mismatch"),
                        sub:
                            index === undefined
                                ? text("sub.mismatchNoEntry")
                                : text("sub.mismatch", {index}),
                    }
            }
        default:
            return {head: text(`state.${status}`), sub: text(`sub.${status}`)}
    }
}

const STEP_NAMES: Record<EStatusStep, string> = {
    [EStatusStep.RECORD]: "record",
    [EStatusStep.WITNESSES]: "witnesses",
    [EStatusStep.ANCHORS]: "anchors",
}

const failedDetail = (
    step: EStatusStep,
    problem: EBallotProblem | undefined,
    index: number | undefined
): ITranslation => {
    switch (problem) {
        case EBallotProblem.DAMAGED:
            return text("row.damaged")
        case EBallotProblem.NO_RECEIPT:
            return text("row.noReceipt")
        case EBallotProblem.UNCHECKED:
            return text("row.unchecked")
        default:
            if (step !== EStatusStep.RECORD) return text("row.contradiction")
            return index === undefined ? text("row.mismatchNoEntry") : text("row.mismatch", {index})
    }
}

const FAILED_WORDS: Record<EBallotProblem, string> = {
    [EBallotProblem.MISMATCH]: "mismatch",
    [EBallotProblem.DAMAGED]: "damaged",
    [EBallotProblem.NO_RECEIPT]: "noReceipt",
    [EBallotProblem.UNCHECKED]: "unchecked",
}

/** The hubs part of the anchors, with the browser layer when the policy has one. */
const anchoredDetail = (result: ChainResult | undefined): ITranslation => {
    const hubs = result?.policy.hubs.length ?? 0
    if (hubs === 0) return text("step.anchors.upcoming")
    const layer = result?.policy.browser
    if (layer === "waict") return text("row.anchoredWaict", {count: hubs})
    if (layer === "bootstrap") return text("row.anchoredBootstrap", {count: hubs})
    return text("row.anchoredHubs", {count: hubs})
}

const doneDetail = (step: EStatusStep, result: ChainResult | undefined): ITranslation => {
    switch (step) {
        case EStatusStep.RECORD: {
            const index = result?.entry?.index
            return index === undefined ? text("row.recorded") : text("row.recordedEntry", {index})
        }
        case EStatusStep.WITNESSES: {
            const count = result?.policy.witnesses.length ?? 0
            return count > 0 ? text("row.witnessed", {count}) : text("step.witnesses.upcoming")
        }
        default:
            return anchoredDetail(result)
    }
}

const lateDetail = (step: EStatusStep, result: ChainResult | undefined): ITranslation => {
    const deadlines = (result?.tiers ?? [])
        .filter((t) => STEP_TIERS[step].includes(t.tier) && t.status === "late")
        .flatMap((t) => (t.deadline === undefined ? [] : [t.deadline]))
    const deadline = deadlines.length > 0 ? Math.min(...deadlines) : undefined
    if (deadline === undefined) return text("row.lateNow")
    return step === EStatusStep.ANCHORS
        ? text("row.lateAnchors", {time: clockMinutes(deadline)})
        : text("row.lateStep", {time: clockMinutes(deadline)})
}

/** The three steps, each with what it means now and the receipt's facts. */
export const statusRows = (input: IStatusInput): IStatusRow[] => {
    const {status, problem, result, policy, times} = input
    const states = statusSteps(status, result)
    const minutes = anchorMinutes(policy)
    return STATUS_STEPS.map((step): IStatusRow => {
        const state = states[step]
        const base = {
            step,
            state,
            title: text(`step.${STEP_NAMES[step]}.title`),
        }
        const anchors = step === EStatusStep.ANCHORS
        switch (state) {
            case EStepState.UPCOMING:
                return {...base, detail: text(`step.${STEP_NAMES[step]}.upcoming`)}
            case EStepState.WAITING:
                return {
                    ...base,
                    detail: text("row.waiting"),
                    status: anchors
                        ? text("word.withinMinutes", {minutes})
                        : text("word.withinSeconds"),
                }
            case EStepState.ACTIVE:
                return {
                    ...base,
                    detail: anchors ? text("row.anchoring", {minutes}) : text("row.checking"),
                    status: anchors ? text("word.withinMinutes", {minutes}) : text("word.checking"),
                }
            case EStepState.DONE: {
                const doneAt = times?.[step]
                return {
                    ...base,
                    detail: doneDetail(step, result),
                    ...(doneAt === undefined ? {} : {doneAt}),
                }
            }
            case EStepState.LATE:
                return {
                    ...base,
                    detail: lateDetail(step, result),
                    status: text("word.late"),
                }
            case EStepState.FAILED:
                return {
                    ...base,
                    detail: failedDetail(step, problem, result?.entry?.index),
                    status: text(`word.${FAILED_WORDS[problem ?? EBallotProblem.MISMATCH]}`),
                }
            default:
                return {
                    ...base,
                    detail:
                        problem === EBallotProblem.MISMATCH
                            ? text("row.skippedMismatch")
                            : text("row.skipped"),
                    status: text("word.skipped"),
                }
        }
    })
}

/** The caption under the steps, as keys to show one after the other. */
export const statusCaption = (status: EBallotStatus, kept: boolean): ITranslation[] => {
    switch (status) {
        case EBallotStatus.NOT_CAST:
            return [text("caption.notcast")]
        case EBallotStatus.CHECKING:
            return [text("caption.checking")]
        case EBallotStatus.PROBLEM:
        case EBallotStatus.AUDIT_FAILED:
            return [text("caption.problem")]
        case EBallotStatus.AUDITABLE:
            return [text("caption.auditable")]
        case EBallotStatus.VERIFIED:
            return [text("caption.verified"), ...(kept ? [text("caption.kept")] : [])]
        default:
            return [text("caption.checked"), ...(kept ? [text("caption.kept")] : [])]
    }
}
