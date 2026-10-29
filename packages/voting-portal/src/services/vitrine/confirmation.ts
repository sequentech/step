// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the voter is told around the receipt (Vitrine's design boards Main,
// Revisit and Alert): where the ballot stands, derived only from the receipt's
// notice and the verifier's result, so the page never says more than the
// checks do.

import type {ChainResult, Reason, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EReceiptNotice} from "./castReceipt"

export enum EConfirmationPhase {
    /** Cast, but the record's witnesses haven't confirmed it yet. */
    CAST = "cast",
    /** The record includes the ballot and independent witnesses signed it. */
    RECORDED = "recorded",
    /** The receipt owed never came, or signed proofs contradict each other. */
    FAILED = "failed",
}

const tierOf = (result: ChainResult | undefined, tier: TierName): TierResult | undefined =>
    result?.tiers.find((t) => t.tier === tier)

/** Where the ballot stands. A result that is only `invalid` is damaged data,
 * not evidence, so it never reads as a failure. */
export const confirmationPhase = (
    notice: EReceiptNotice,
    result: ChainResult | undefined
): EConfirmationPhase => {
    if (notice === EReceiptNotice.MISSING || result?.status === "failed") {
        return EConfirmationPhase.FAILED
    }
    const included = tierOf(result, "record")?.status === "verified"
    const witnessed = tierOf(result, "witnesses")?.status === "verified"
    return included && witnessed ? EConfirmationPhase.RECORDED : EConfirmationPhase.CAST
}

export enum ERevisitPhase {
    /** Included, witnessed and unchanged, as far as the tiers say. */
    HOLDS = "holds",
    /** Some proofs are still on their way. */
    WAITING = "waiting",
    /** Signed proofs contradict each other. */
    PROBLEM = "problem",
    /** The receipt couldn't be checked, or hasn't been yet. */
    UNCHECKED = "unchecked",
}

export interface IRevisitSummary {
    phase: ERevisitPhase
    witnesses?: TierStatus
    browser?: TierStatus
    /** The app hubs that hold the record, of all those the policy names. */
    hubs?: {agree: number; total: number; status: TierStatus}
}

const VERIFIED: TierStatus = "verified"

/** What a kept receipt says now: the tiers a voter can follow, and whether
 * the record still holds the ballot. */
export const revisitSummary = (result: ChainResult | undefined): IRevisitSummary => {
    if (!result) return {phase: ERevisitPhase.UNCHECKED}
    const witnesses = tierOf(result, "witnesses")
    const hubs = tierOf(result, "hubs")
    const browser = tierOf(result, "browser")
    const summary: IRevisitSummary = {phase: ERevisitPhase.UNCHECKED}
    if (witnesses) summary.witnesses = witnesses.status
    if (browser) summary.browser = browser.status
    if (hubs) {
        const details = hubs.hubs ?? []
        summary.hubs = {
            agree: details.filter((h) => h.status === VERIFIED).length,
            total: details.length,
            status: hubs.status,
        }
    }
    if (result.status === "failed") summary.phase = ERevisitPhase.PROBLEM
    else if (result.status === "invalid") summary.phase = ERevisitPhase.UNCHECKED
    else if (
        tierOf(result, "record")?.status === VERIFIED &&
        witnesses?.status === VERIFIED &&
        result.facts.unchanged === "done"
    ) {
        summary.phase = ERevisitPhase.HOLDS
    } else summary.phase = ERevisitPhase.WAITING
    return summary
}

export interface IProblem {
    tier: TierName
    reasons: Reason[]
}

export interface IFailure {
    /** The ballot's entry in the log, when the receipt says it. */
    entry?: {log: string; index?: number}
    /** The tiers whose signed proofs contradict each other. */
    problems: IProblem[]
}

/** The evidence a failed result holds: the receipt's entry and each tier
 * that failed, with the verifier's reasons. Nothing for a result that isn't
 * `failed`: damaged data is not evidence. */
export const failureOf = (result: ChainResult | undefined): IFailure | undefined => {
    if (result?.status !== "failed") return undefined
    const failure: IFailure = {
        problems: result.tiers
            .filter((t) => t.status === "failed")
            .map((t) => ({tier: t.tier, reasons: t.reasons})),
    }
    if (result.entry) {
        failure.entry = {log: result.entry.log}
        if (result.entry.index !== undefined) failure.entry.index = result.entry.index
    }
    return failure
}

/** A file name for the receipt the voter saves. */
export const receiptFileName = (ballotId: string): string =>
    `receipt-${ballotId.replace(/[^A-Za-z0-9]/g, "").slice(0, 8) || "vitrine"}.vitrine-receipt`
