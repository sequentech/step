// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The receipt screen's checks (Vitrine's specs/receipt.md, sections 1.2 and
// 3.1). The first round asks only the policy's logs, at once. The full round
// also asks the log's mirrors, the anchor hubs and the monitors, after a
// random delay of 1 to 30 minutes, so that those third parties can't tie
// their requests to the moment the receipt was opened; it runs at once when
// an anchor hub or the browser layer is late, or when the user asks. A
// private event's logs serve no one, so its receipt is checked with what the
// file holds.

import type {BallotStage, ChainResult, Expect, PolicyHistory, ReceiptReference} from "@vitrine/verify"
import {FetchLike, sdkFetch} from "./http"
import type {IPolicyView} from "./policies"
import type {IVitrineSdk} from "./sdk"

export enum ECheckRound {
    /** The policy's logs only. */
    LOG = "log",
    /** The logs, their mirrors, the anchor hubs and the monitors. */
    FULL = "full",
    /** Nothing fetched: a private event's receipt. */
    OFFLINE = "offline",
}

const MINUTE_MS = 60_000
export const MIN_FULL_ROUND_DELAY_MS = MINUTE_MS
export const MAX_FULL_ROUND_DELAY_MS = 30 * MINUTE_MS

/** When the full round runs after the first, in milliseconds. */
export const fullRoundDelay = (random: () => number): number =>
    MIN_FULL_ROUND_DELAY_MS + random() * (MAX_FULL_ROUND_DELAY_MS - MIN_FULL_ROUND_DELAY_MS)

export interface ICheckTarget {
    chain: string
    view: IPolicyView
    expect: Expect
    /** The pinned policy and the successors found so far, kept across rounds. */
    history: PolicyHistory
}

export interface ICheckOptions {
    fetch?: FetchLike
    /** The device clock, POSIX seconds. */
    now?: () => number
    signal?: AbortSignal
}

export interface ICheckOutcome {
    /** The chain as far as the round upgraded it. */
    chain: string
    result: ChainResult
    round: ECheckRound
}

const deviceNow = (): number => Math.floor(Date.now() / 1000)

/** The base URLs of the policy's logs. */
export const logUrls = (view: IPolicyView): string[] =>
    Array.from(view.policy.logs.values()).flatMap((log) => (log.url ? [log.url] : []))

export const isPrivate = (view: IPolicyView): boolean => view.policy.visibility === "private"

/** True when the result can't wait for the full round: an anchor hub or the
 * browser layer is late, or a hub holds an older record than the receipt. */
export const needsFullRoundNow = (result: ChainResult): boolean =>
    result.tiers.some(
        (t) =>
            (t.tier === "hubs" || t.tier === "browser") &&
            (t.status === "late" ||
                (t.hubs ?? []).some(
                    (h) =>
                        h.status === "late" ||
                        h.reasons.some((r) => r.code === "anchor-behind-record")
                ))
    )

/** Runs one round on the chain: the log-only round or the full round, or,
 * for a private event, the offline check. */
export const checkRound = async (
    sdk: IVitrineSdk,
    target: ICheckTarget,
    round: ECheckRound.LOG | ECheckRound.FULL,
    opts: ICheckOptions = {}
): Promise<ICheckOutcome> => {
    const {chain, view, expect, history} = target
    const now = opts.now ?? deviceNow
    if (isPrivate(view)) {
        const result = sdk.verify.verifyChain(chain, view.policy, {
            expect,
            where: "independent",
            now: now(),
            history,
        })
        return {chain, result, round: ECheckRound.OFFLINE}
    }
    const logOnly = round === ECheckRound.LOG
    const checked = await sdk.verify.checkChain(chain, view.policy, {
        expect,
        where: "independent",
        history,
        now,
        fetch: sdkFetch(opts.fetch, logOnly ? logUrls(view) : undefined),
        // The page has already waited before the full round.
        thirdPartyDelay: () => 0,
        ...(logOnly ? {checkMirror: false, hubCheckpoints: false, gossip: false} : {}),
        ...(opts.signal ? {signal: opts.signal} : {}),
    })
    return {chain: checked.chain, result: checked.result, round}
}

/** Rebuilds the chain of a receipt reference from the policy's log only. */
export const chainFromReference = (
    sdk: IVitrineSdk,
    reference: ReceiptReference,
    view: IPolicyView,
    history: PolicyHistory,
    opts: ICheckOptions = {}
): Promise<string> =>
    sdk.verify.chainFromReference(reference, view.policy, {
        history,
        fetch: sdkFetch(opts.fetch, logUrls(view)),
        ...(opts.now ? {now: opts.now} : {}),
        ...(opts.signal ? {signal: opts.signal} : {}),
    })

export interface IBallotForm {
    eventId: string
    electionId: string
    ballotId: string
    /** The step of the ballot the receipt is for: its entry holds it. */
    stage: BallotStage
}

/** The expectation a Tracking ID, a step, an election and an event give: the
 * entry must be that ballot's at that step, with the `accepted_at` the receipt
 * holds (specs/receipt.md, section 1.1). Undefined when they can't make an
 * entry. The whole Tracking ID is needed, not a short one. */
export const expectFromForm = (sdk: IVitrineSdk, form: IBallotForm): Expect | undefined => {
    const eventId = form.eventId.trim()
    const electionId = form.electionId.trim()
    if (!eventId || !electionId) return undefined
    try {
        return sdk.verify.ballotExpect({
            eventId,
            electionId,
            stage: form.stage,
            ballotId: sdk.verify.decodeTrackingId(form.ballotId.trim()),
        })
    } catch {
        return undefined
    }
}

/** The Tracking ID an expectation names, to show the user. */
export const expectedBallotId = (sdk: IVitrineSdk, expect: Expect): string | undefined => {
    try {
        if ("fields" in expect) return sdk.verify.trackingId(expect.fields.ballot_id)
        const id = sdk.pure.entries.fieldValue(
            sdk.pure.entries.parseEntry(expect.entry),
            "ballot_id"
        )
        return id && sdk.verify.trackingId(id)
    } catch {
        return undefined
    }
}
