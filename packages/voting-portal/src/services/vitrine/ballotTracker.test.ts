// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, Policy, Step, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EBallotIdInput} from "./ballotId"
import {EBallotStatus} from "./ballotStatus"
import {
    acceptedAtOf,
    ballotLookupOf,
    ETrackerState,
    ETrackerStep,
    ETrackerView,
    rejectedBallotId,
    RING_PROGRESS,
    ringProgress,
    trackedResult,
    trackerInput,
    trackerStatusOf,
    trackerSteps,
    trackerViewOf,
} from "./ballotTracker"
import {EReceiptNotice, IReceiptView} from "./castReceipt"

const FULL = "30ffd31442386472fd6dbdb4df3695590f7a414d02ab24995befc5a940b89da1"
const NOW = 1_793_270_528
const WITNESS_NAMES = ["a", "b", "c", "d", "e"]

const tier = (name: TierName, status: TierStatus, extra: Partial<TierResult> = {}): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...extra,
})

const QUORUM_STEP: Step = {
    code: "record-quorum",
    tier: "witnesses",
    ok: true,
    params: {quorumTime: NOW + 1, cosignatures: 5},
}

const resultOf = (
    status: ChainResult["status"],
    tiers: TierResult[],
    extra: Partial<ChainResult> = {}
): ChainResult =>
    ({
        ok: status !== "failed" && status !== "invalid",
        status,
        tiers,
        steps: [QUORUM_STEP],
        entry: {
            log: "ballots",
            leafHash: "b1d8c47561320000000000000000000000000000000000000000000000005a0d",
            index: 21,
            promised: false,
        },
        policy: {
            id: "p",
            words: [],
            lang: "en",
            event: "e",
            witnesses: [],
            hubs: ["hub-a", "hub-b"],
        },
        now: NOW,
        ...extra,
    }) as unknown as ChainResult

const POLICY = {
    witnesses: {
        witnesses: new Map(WITNESS_NAMES.map((name) => [name, {name}])),
        groups: new Map([["quorum", {name: "quorum", k: 4, members: WITNESS_NAMES}]]),
        quorum: "quorum",
    },
} as unknown as Policy

const viewOf = (
    result: ChainResult | undefined,
    extra: Partial<IReceiptView> = {}
): IReceiptView => ({
    notice: EReceiptNotice.NONE,
    policy: POLICY,
    chain: "chain",
    own: {entry: new Uint8Array(), acceptedAt: NOW},
    ...(result ? {result} : {}),
    ...extra,
})

const VERIFIED = resultOf("verified", [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified"),
    tier("browser", "verified"),
])

const states = (view: IReceiptView | undefined) => trackerSteps(view).map((s) => s.state)

describe("looking a ballot up from its address", () => {
    it("looks a short ID up by prefix, in any spacing and case", () => {
        expect(ballotLookupOf("30ffd314", false)).toEqual({ballotIdPrefix: "30ffd314"})
        expect(ballotLookupOf("30FF d314", false)).toEqual({ballotIdPrefix: "30ffd314"})
    })

    it("looks a full ID up as it is", () => {
        expect(ballotLookupOf(FULL, false)).toEqual({ballotId: FULL})
    })

    it("takes the first four characters only for telephone voting", () => {
        expect(ballotLookupOf("abcd", true)).toEqual({ballotIdPrefix: "abcd"})
        expect(ballotLookupOf("abcd", false)).toEqual({ballotId: "abcd"})
    })

    it("looks nothing up for what isn't hex", () => {
        expect(ballotLookupOf("30ffd31g", false)).toBeNull()
        expect(ballotLookupOf("", false)).toBeNull()
        expect(ballotLookupOf(undefined, false)).toBeNull()
    })
})

describe("what the search field holds", () => {
    it("takes the short ID and the full one, with or without spaces", () => {
        expect(trackerInput("30ff d314", false)).toBe(EBallotIdInput.SHORT)
        expect(trackerInput("30ffd314", false)).toBe(EBallotIdInput.SHORT)
        expect(trackerInput(FULL, false)).toBe(EBallotIdInput.FULL)
    })

    it("refuses anything between, but telephone events take four characters", () => {
        expect(trackerInput("30ff", false)).toBe(EBallotIdInput.INVALID)
        expect(trackerInput("30ff", true)).toBe(EBallotIdInput.SHORT)
        expect(trackerInput("30ff d3", true)).toBe(EBallotIdInput.INVALID)
        expect(trackerInput("", true)).toBe(EBallotIdInput.EMPTY)
    })

    it("notices characters the field leaves out", () => {
        expect(rejectedBallotId("30ff d314")).toBe(false)
        expect(rejectedBallotId("")).toBe(false)
        expect(rejectedBallotId("30ff d31g")).toBe(true)
        expect(rejectedBallotId(`${FULL}0`)).toBe(true)
    })
})

describe("when a ballot was accepted", () => {
    it("reads the ballot box's time as the log's whole seconds", () => {
        expect(acceptedAtOf("2026-10-29T10:42:08.987654+00:00")).toBe(
            Date.UTC(2026, 9, 29, 10, 42, 8) / 1000
        )
        expect(acceptedAtOf("2026-10-29T12:42:08+02:00")).toBe(
            Date.UTC(2026, 9, 29, 10, 42, 8) / 1000
        )
    })

    it("knows nothing from a missing or damaged time", () => {
        expect(acceptedAtOf(undefined)).toBeUndefined()
        expect(acceptedAtOf(null)).toBeUndefined()
        expect(acceptedAtOf("yesterday")).toBeUndefined()
    })
})

describe("the status block's steps", () => {
    it("checks the record first while there is nothing to show", () => {
        expect(states(undefined)).toEqual([
            ETrackerState.ACTIVE,
            ETrackerState.WAITING,
            ETrackerState.WAITING,
            ETrackerState.WAITING,
        ])
        expect(trackerStatusOf(undefined)).toBe(EBallotStatus.CHECKING)
    })

    it("shows every step done for a verified ballot, with what the result says", () => {
        const steps = trackerSteps(viewOf(VERIFIED))
        expect(steps.map((s) => s.state)).toEqual(Array(4).fill(ETrackerState.DONE))
        expect(steps[0]).toMatchObject({step: ETrackerStep.RECORD, at: NOW, index: 21})
        expect(steps[1]).toMatchObject({
            at: NOW + 1,
            witnesses: {signed: 5, total: 5, needed: 4},
        })
        expect(steps[2].hubs).toEqual(["hub-a", "hub-b"])
        expect(trackerStatusOf(viewOf(VERIFIED))).toBe(EBallotStatus.VERIFIED)
    })

    it("leaves out the number a quorum of groups doesn't give", () => {
        const nested = {
            witnesses: {
                witnesses: new Map(WITNESS_NAMES.map((name) => [name, {name}])),
                groups: new Map([["quorum", {name: "quorum", k: 2, members: ["east", "west"]}]]),
                quorum: "quorum",
            },
        } as unknown as Policy
        const [, witnesses] = trackerSteps(viewOf(VERIFIED, {policy: nested}))
        expect(witnesses.witnesses).toEqual({signed: 5, total: 5})
    })

    it("schedules anchors that are due later, with the minutes left", () => {
        const result = resultOf("pending", [
            tier("record", "verified"),
            tier("witnesses", "verified"),
            tier("hubs", "verified"),
            tier("browser", "pending", {deadline: NOW + 600}),
        ])
        const steps = trackerSteps(viewOf(result))
        expect(steps.map((s) => s.state)).toEqual([
            ETrackerState.DONE,
            ETrackerState.DONE,
            ETrackerState.DONE,
            ETrackerState.SCHEDULED,
        ])
        expect(steps[3]).toMatchObject({due: NOW + 600, minutes: 10})
        expect(trackerStatusOf(viewOf(result))).toBe(EBallotStatus.WITNESSED)
    })

    it("marks a late step with when it was due, and the steps after it wait", () => {
        const result = resultOf("late", [
            tier("record", "verified"),
            tier("witnesses", "verified"),
            tier("hubs", "late", {deadline: NOW - 60}),
            tier("browser", "pending"),
        ])
        const steps = trackerSteps(viewOf(result))
        expect(steps.map((s) => s.state)).toEqual([
            ETrackerState.DONE,
            ETrackerState.DONE,
            ETrackerState.LATE,
            ETrackerState.WAITING,
        ])
        expect(steps[2].due).toBe(NOW - 60)
        expect(trackerStatusOf(viewOf(result))).toBe(EBallotStatus.LATE)
    })

    it("fails the record and checks nothing after it", () => {
        const result = resultOf("failed", [
            tier("record", "failed"),
            tier("witnesses", "pending"),
            tier("hubs", "pending"),
            tier("browser", "pending"),
        ])
        expect(states(viewOf(result))).toEqual([
            ETrackerState.FAILED,
            ETrackerState.SKIPPED,
            ETrackerState.SKIPPED,
            ETrackerState.SKIPPED,
        ])
        expect(trackerStatusOf(viewOf(result))).toBe(EBallotStatus.PROBLEM)
    })

    it("leaves out the layers a policy doesn't use", () => {
        const result = resultOf("verified", [
            tier("record", "verified"),
            tier("witnesses", "verified"),
            tier("hubs", "unused"),
            tier("browser", "unused"),
        ])
        expect(trackerSteps(viewOf(result)).map((s) => s.step)).toEqual([
            ETrackerStep.RECORD,
            ETrackerStep.WITNESSES,
        ])
    })

    it("takes damaged data for no result", () => {
        const damaged = viewOf(resultOf("invalid", [tier("record", "invalid")]))
        expect(trackedResult(damaged)).toBeUndefined()
        expect(trackerStatusOf(damaged)).toBe(EBallotStatus.CHECKING)
    })
})

describe("the rings", () => {
    it("fills a ring for a step with an outcome, part of one being checked", () => {
        expect(ringProgress(ETrackerState.DONE)).toBe(RING_PROGRESS.FULL)
        expect(ringProgress(ETrackerState.LATE)).toBe(RING_PROGRESS.FULL)
        expect(ringProgress(ETrackerState.FAILED)).toBe(RING_PROGRESS.FULL)
        expect(ringProgress(ETrackerState.ACTIVE)).toBe(RING_PROGRESS.CHECKING)
        expect(ringProgress(ETrackerState.SCHEDULED)).toBe(RING_PROGRESS.NONE)
        expect(ringProgress(undefined)).toBe(RING_PROGRESS.NONE)
    })
})

describe("what the status block shows", () => {
    it("shows the status once there is a result", () => {
        expect(trackerViewOf({view: viewOf(VERIFIED), kept: true, settled: true})).toBe(
            ETrackerView.STATUS
        )
        expect(trackerViewOf({view: viewOf(VERIFIED), kept: false, settled: false})).toBe(
            ETrackerView.STATUS
        )
    })

    it("keeps checking until the lookup is over", () => {
        expect(trackerViewOf({kept: false, settled: false})).toBe(ETrackerView.CHECKING)
        expect(trackerViewOf({kept: true, settled: false})).toBe(ETrackerView.CHECKING)
    })

    it("says so when this device has no receipt and the log gave none", () => {
        expect(trackerViewOf({kept: false, settled: true})).toBe(ETrackerView.NO_RECEIPT)
        const waiting = viewOf(undefined, {notice: EReceiptNotice.WAITING})
        expect(trackerViewOf({view: waiting, kept: false, settled: true})).toBe(
            ETrackerView.NO_RECEIPT
        )
    })

    it("tells a kept receipt that waits from one that can't be checked", () => {
        const waiting = viewOf(undefined, {notice: EReceiptNotice.WAITING})
        expect(trackerViewOf({view: waiting, kept: true, settled: true})).toBe(ETrackerView.WAITING)
        const unchecked = viewOf(undefined, {notice: EReceiptNotice.UNCHECKED})
        expect(trackerViewOf({view: unchecked, kept: true, settled: true})).toBe(
            ETrackerView.UNCHECKED
        )
        const damaged = viewOf(resultOf("invalid", [tier("record", "invalid")]))
        expect(trackerViewOf({view: damaged, kept: true, settled: true})).toBe(
            ETrackerView.UNCHECKED
        )
    })
})
