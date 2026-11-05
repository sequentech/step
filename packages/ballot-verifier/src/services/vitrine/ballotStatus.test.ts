// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, OverallStatus, Policy, TierName, TierStatus} from "@vitrine/verify"
import {
    ballotProblemOf,
    ballotStatusOf,
    EBallotProblem,
    EBallotStatus,
    EStatusStep,
    EStepState,
    statusFacts,
    statusSteps,
} from "./ballotStatus"

const resultOf = (
    status: OverallStatus,
    tiers: Partial<Record<TierName, TierStatus>>,
    rest: Partial<ChainResult> = {}
): ChainResult =>
    ({
        status,
        tiers: Object.entries(tiers).map(([tier, s]) => ({tier, status: s, reasons: []})),
        steps: [],
        policy: {witnesses: []},
        ...rest,
    }) as unknown as ChainResult

const PENDING = {
    record: "pending",
    witnesses: "pending",
    hubs: "pending",
    browser: "pending",
} as const

const VERIFIED = {
    record: "verified",
    witnesses: "verified",
    hubs: "verified",
    browser: "verified",
} as const

describe("ballotStatusOf", () => {
    it("checks until there is a result", () => {
        expect(ballotStatusOf(undefined)).toBe(EBallotStatus.CHECKING)
        expect(ballotStatusOf(resultOf("checking", {}))).toBe(EBallotStatus.CHECKING)
        expect(ballotStatusOf(resultOf("pending", PENDING))).toBe(EBallotStatus.CHECKING)
    })

    it("climbs from the record to the witnesses to verified", () => {
        const recorded = resultOf("pending", {...PENDING, record: "verified"})
        expect(ballotStatusOf(recorded)).toBe(EBallotStatus.RECORDED)
        const witnessed = resultOf("pending", {
            ...PENDING,
            record: "verified",
            witnesses: "verified",
        })
        expect(ballotStatusOf(witnessed)).toBe(EBallotStatus.WITNESSED)
        expect(ballotStatusOf(resultOf("verified", VERIFIED))).toBe(EBallotStatus.VERIFIED)
    })

    it("counts a tier the policy doesn't use as done", () => {
        const noWitnesses = resultOf("pending", {
            ...PENDING,
            record: "verified",
            witnesses: "unused",
        })
        expect(ballotStatusOf(noWitnesses)).toBe(EBallotStatus.WITNESSED)
    })

    it("shows late and problems over progress", () => {
        const late = resultOf("late", {record: "verified", witnesses: "verified", hubs: "late"})
        expect(ballotStatusOf(late)).toBe(EBallotStatus.LATE)
        const failed = resultOf("failed", {record: "failed"})
        expect(ballotStatusOf(failed)).toBe(EBallotStatus.PROBLEM)
        expect(ballotProblemOf(failed)).toBe(EBallotProblem.MISMATCH)
        const invalid = resultOf("invalid", {record: "verified", witnesses: "invalid"})
        expect(ballotStatusOf(invalid)).toBe(EBallotStatus.PROBLEM)
        expect(ballotProblemOf(invalid)).toBe(EBallotProblem.DAMAGED)
        expect(ballotProblemOf(resultOf("verified", VERIFIED))).toBeUndefined()
        expect(ballotProblemOf(undefined)).toBeUndefined()
    })
})

describe("statusSteps", () => {
    it("checks the record first, without a result", () => {
        expect(statusSteps()).toEqual({
            record: EStepState.ACTIVE,
            witnesses: EStepState.WAITING,
            anchors: EStepState.WAITING,
        })
    })

    it("makes each step wait for the ones before it", () => {
        const result = resultOf("pending", {
            record: "pending",
            witnesses: "verified",
            hubs: "verified",
        })
        expect(statusSteps(result)).toEqual({
            record: EStepState.ACTIVE,
            witnesses: EStepState.WAITING,
            anchors: EStepState.WAITING,
        })
    })

    it("joins the hubs and the browser layer into the anchors", () => {
        const late = resultOf("late", {...VERIFIED, browser: "late"})
        expect(statusSteps(late).anchors).toBe(EStepState.LATE)
        const active = resultOf("pending", {...VERIFIED, browser: "pending"})
        expect(statusSteps(active).anchors).toBe(EStepState.ACTIVE)
        expect(statusSteps(resultOf("verified", VERIFIED)).anchors).toBe(EStepState.DONE)
    })

    it("skips the steps after one that failed", () => {
        const failed = resultOf("failed", {
            record: "verified",
            witnesses: "failed",
            hubs: "verified",
        })
        expect(statusSteps(failed)).toEqual({
            record: EStepState.DONE,
            witnesses: EStepState.FAILED,
            anchors: EStepState.SKIPPED,
        })
    })

    it("takes a record that isn't the entry looked for as a mismatch, not damage", () => {
        const other = resultOf("invalid", {
            record: "invalid",
            witnesses: "invalid",
            hubs: "invalid",
        })
        other.tiers[0].reasons = [{code: "entry-mismatch"}]
        expect(ballotProblemOf(other)).toBe(EBallotProblem.MISMATCH)
        expect(statusSteps(other)).toEqual({
            record: EStepState.FAILED,
            witnesses: EStepState.SKIPPED,
            anchors: EStepState.SKIPPED,
        })
    })

    it("tells damaged data from a mismatch", () => {
        const damaged = resultOf("invalid", {record: "invalid", witnesses: "verified"})
        expect(statusSteps(damaged)).toEqual({
            record: EStepState.DAMAGED,
            witnesses: EStepState.SKIPPED,
            anchors: EStepState.SKIPPED,
        })
    })
})

describe("statusFacts", () => {
    const policyOf = (members: string[], k: number, witnesses = members): Policy =>
        ({
            witnesses: {
                witnesses: new Map(witnesses.map((name) => [name, {}])),
                groups: new Map([["quorum", {k, members}]]),
                quorum: "quorum",
            },
        }) as unknown as Policy

    const hub = (status: TierStatus, test = false) => ({hub: "h", status, test, reasons: []})

    it("counts the witnesses that signed, those the policy names and its quorum", () => {
        const result = resultOf("verified", VERIFIED, {
            entry: {log: "ballots", leafHash: "", index: 21, promised: false},
            steps: [
                {code: "record-quorum", tier: "witnesses", ok: true, params: {cosignatures: 9}},
            ],
            policy: {witnesses: [{name: "w1"}, {name: "w2"}, {name: "w3"}]},
        } as unknown as Partial<ChainResult>)
        expect(statusFacts(result, policyOf(["w1", "w2", "w3"], 2))).toMatchObject({
            index: 21,
            signed: 3,
            witnesses: 3,
            needed: 2,
        })
    })

    it("leaves out what the result doesn't say", () => {
        const facts = statusFacts(resultOf("pending", PENDING), policyOf(["g"], 1, ["w1"]))
        expect(facts.index).toBeUndefined()
        expect(facts.signed).toBeUndefined()
        expect(facts.witnesses).toBeUndefined()
        expect(facts.needed).toBeUndefined()
        expect(facts.anchorsDue).toBeUndefined()
    })

    it("counts the public hubs that hold the record, not the test ones", () => {
        const result = resultOf("pending", {...PENDING, hubs: "pending"})
        result.tiers[2] = {
            tier: "hubs",
            status: "pending",
            hubs: [hub("verified"), hub("verified", true), hub("pending")],
            reasons: [],
        }
        expect(statusFacts(result, policyOf([], 1)).hubs).toBe(1)
    })

    it("says when late anchors were due, and which steps the policy doesn't use", () => {
        const result = resultOf("late", {
            record: "verified",
            witnesses: "unused",
            hubs: "late",
            browser: "late",
        })
        result.tiers[2] = {tier: "hubs", status: "late", deadline: 1_800_000_200, reasons: []}
        result.tiers[3] = {tier: "browser", status: "late", deadline: 1_800_000_100, reasons: []}
        const facts = statusFacts(result, policyOf([], 1))
        expect(facts.anchorsDue).toBe(1_800_000_100)
        expect(facts.unused).toEqual([EStatusStep.WITNESSES])
    })
})
