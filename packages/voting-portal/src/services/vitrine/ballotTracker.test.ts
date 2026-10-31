// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, Policy, Step, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EBallotIdInput} from "./ballotId"
import {EBallotStatus} from "./ballotStatus"
import {
    acceptedAtOf,
    ballotLookupOf,
    classifyRecord,
    ERecordEncoding,
    ERecordKind,
    ETrackerState,
    ETrackerStep,
    ETrackerView,
    recordFileName,
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

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"
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
        expect(ballotLookupOf("FTBEMHRX")).toEqual({ballotIdPrefix: "FTBEMHRX"})
        expect(ballotLookupOf("ftbe-mhrx")).toEqual({ballotIdPrefix: "FTBEMHRX"})
    })

    it("looks a full ID up as it is", () => {
        expect(ballotLookupOf(FULL)).toEqual({ballotId: FULL})
    })

    it("looks the start of an ID up by prefix from 8 characters, and nothing shorter", () => {
        expect(ballotLookupOf("FTBE-MHRX-1")).toEqual({ballotIdPrefix: "FTBEMHRX1"})
        expect(ballotLookupOf("FTBE")).toBeNull()
        expect(ballotLookupOf("FTBE-MHR")).toBeNull()
    })

    it("looks nothing up for what isn't a Tracking ID, a voter pseudonym among it", () => {
        expect(ballotLookupOf("FTBEMHRU")).toBeNull()
        expect(ballotLookupOf("FTBE:MHRX")).toBeNull()
        expect(ballotLookupOf("")).toBeNull()
        expect(ballotLookupOf(undefined)).toBeNull()
    })
})

describe("what the search field holds", () => {
    it("takes the short ID and the full one, with or without hyphens", () => {
        expect(trackerInput("FTBE-MHRX")).toBe(EBallotIdInput.SHORT)
        expect(trackerInput("ftbemhrx")).toBe(EBallotIdInput.SHORT)
        expect(trackerInput(FULL)).toBe(EBallotIdInput.FULL)
    })

    it("takes the start of an ID from 8 characters and refuses less", () => {
        expect(trackerInput("FTBE-MHRX-1")).toBe(EBallotIdInput.PARTIAL)
        expect(trackerInput("FTBE")).toBe(EBallotIdInput.INVALID)
        expect(trackerInput("FTBE-MH")).toBe(EBallotIdInput.INVALID)
        expect(trackerInput("")).toBe(EBallotIdInput.EMPTY)
    })

    it("does not take a voter pseudonym for a Tracking ID", () => {
        expect(trackerInput("FTBE:MHRX")).toBe(EBallotIdInput.INVALID)
        expect(trackerInput("FTBE:MHRX")).toBe(EBallotIdInput.INVALID)
    })

    it("notices characters the field leaves out", () => {
        expect(rejectedBallotId("FTBE-MHRX")).toBe(false)
        expect(rejectedBallotId("")).toBe(false)
        expect(rejectedBallotId("FTBE-MHRU")).toBe(true)
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

const hashableBallot = (extra: Record<string, unknown> = {}) => ({
    version: 2,
    issue_date: "30/10/2026",
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    config: "council-style",
    ballot_style_hash: "1dce3614efd2cc5d0ddb8a1c47daf79b",
    voter_signing_pk: null,
    voter_ballot_signature: null,
    ...extra,
})

const auditableBallot = (extra: Record<string, unknown> = {}) => ({
    version: 2,
    issue_date: "30/10/2026",
    config: {id: "council-style", contests: []},
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    ballot_hash: FULL,
    voter_signing_pk: null,
    voter_ballot_signature: null,
    ...extra,
})

describe("what a record holds", () => {
    it("takes a ballot whose config is a string, as the ballot box keeps it, for a cast one", () => {
        expect(classifyRecord(JSON.stringify(hashableBallot()))).toEqual({kind: ERecordKind.CAST})
        expect(classifyRecord(JSON.stringify(hashableBallot({contests: "EQAAAGNv"})))).toEqual({
            kind: ERecordKind.CAST,
        })
    })

    it("takes an auditable ballot, whose config is an object, for an audited one", () => {
        const ballot = auditableBallot()
        expect(classifyRecord(JSON.stringify(ballot))).toEqual({
            kind: ERecordKind.AUDITED,
            encoding: ERecordEncoding.SINGLE_CONTEST,
            ballot,
        })
    })

    it("tells a multi-contest audited ballot by its contests, one string", () => {
        const ballot = auditableBallot({contests: "EQAAAGNvdW5jaWw"})
        expect(classifyRecord(JSON.stringify(ballot))).toEqual({
            kind: ERecordKind.AUDITED,
            encoding: ERecordEncoding.MULTI_CONTEST,
            ballot,
        })
    })

    it("prefers the config's type to the other fields a record carries", () => {
        const cast = hashableBallot({ballot_hash: FULL})
        expect(classifyRecord(JSON.stringify(cast)).kind).toBe(ERecordKind.CAST)
    })

    it.each([
        ["no content", undefined],
        ["null content", null],
        ["empty content", ""],
        ["text", "a ballot, as plain text"],
        ["truncated JSON", '{"config": "council-style", "contests": ['],
        ["a JSON list", "[1, 2, 3]"],
        ["a JSON number", "42"],
        ["JSON null", "null"],
        ["a JSON string", '"30ffd314"'],
        ["an empty object", "{}"],
    ])("cannot read %s", (_name, content) => {
        expect(classifyRecord(content)).toEqual({kind: ERecordKind.UNREADABLE})
    })

    it.each([
        ["a config that is a number", hashableBallot({config: 7})],
        ["a config that is a list", auditableBallot({config: []})],
        ["a config that is null", auditableBallot({config: null})],
        ["a cast ballot without contests", hashableBallot({contests: undefined})],
        ["a cast ballot whose contests are a number", hashableBallot({contests: 3})],
        ["an audited ballot without a hash", auditableBallot({ballot_hash: undefined})],
        ["an audited ballot with an empty hash", auditableBallot({ballot_hash: ""})],
        ["an audited ballot with a hash that is a number", auditableBallot({ballot_hash: 12})],
        ["an audited ballot without contests", auditableBallot({contests: undefined})],
        ["an audited ballot whose contests are a number", auditableBallot({contests: 3})],
        ["an audited ballot whose contests are not text", auditableBallot({contests: [1, 2]})],
    ])("cannot read a record with %s", (_name, record) => {
        expect(classifyRecord(JSON.stringify(record))).toEqual({kind: ERecordKind.UNREADABLE})
    })
})

describe("the file a record is saved as", () => {
    it("is named from the short Tracking ID", () => {
        expect(recordFileName(FULL)).toBe("ballot-FTBEMHRX.json")
        expect(recordFileName("ftbe mhrx 0123")).toBe("ballot-FTBEMHRX.json")
    })

    it("has no ID in its name when there is none", () => {
        expect(recordFileName("")).toBe("ballot.json")
    })
})
