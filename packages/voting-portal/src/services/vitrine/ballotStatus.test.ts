// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, OverallStatus, TierName, TierStatus} from "@vitrine/verify"
import {EReceiptNotice, IReceiptView} from "./castReceipt"
import {
    ballotProblemOf,
    ballotStatusOf,
    ballotTagOf,
    EBallotProblem,
    EBallotStatus,
    EBallotTag,
    EStepState,
    isAuditStatus,
    statusSteps,
} from "./ballotStatus"

const resultOf = (
    status: OverallStatus,
    tiers: Partial<Record<TierName, TierStatus>>
): ChainResult =>
    ({
        status,
        tiers: Object.entries(tiers).map(([tier, s]) => ({tier, status: s, reasons: []})),
    }) as unknown as ChainResult

const viewOf = (result?: ChainResult, notice = EReceiptNotice.NONE): IReceiptView => ({
    notice,
    result,
})

const PENDING = {
    record: "pending",
    witnesses: "pending",
    hubs: "pending",
    browser: "pending",
} as const

describe("ballotStatusOf", () => {
    it("checks until there is a result", () => {
        expect(ballotStatusOf(undefined)).toBe(EBallotStatus.CHECKING)
        expect(ballotStatusOf(viewOf(undefined, EReceiptNotice.WAITING))).toBe(
            EBallotStatus.CHECKING
        )
        expect(ballotStatusOf(viewOf())).toBe(EBallotStatus.CHECKING)
        expect(ballotStatusOf(viewOf(resultOf("checking", {})))).toBe(EBallotStatus.CHECKING)
        expect(ballotStatusOf(viewOf(resultOf("pending", PENDING)))).toBe(EBallotStatus.CHECKING)
    })

    it("climbs from the record to the witnesses to verified", () => {
        const recorded = resultOf("pending", {...PENDING, record: "verified"})
        expect(ballotStatusOf(viewOf(recorded))).toBe(EBallotStatus.RECORDED)
        const witnessed = resultOf("pending", {
            ...PENDING,
            record: "verified",
            witnesses: "verified",
        })
        expect(ballotStatusOf(viewOf(witnessed))).toBe(EBallotStatus.WITNESSED)
        const verified = resultOf("verified", {
            record: "verified",
            witnesses: "verified",
            hubs: "verified",
            browser: "verified",
        })
        expect(ballotStatusOf(viewOf(verified))).toBe(EBallotStatus.VERIFIED)
    })

    it("counts a tier the policy doesn't use as done", () => {
        const noWitnesses = resultOf("pending", {
            ...PENDING,
            record: "verified",
            witnesses: "unused",
        })
        expect(ballotStatusOf(viewOf(noWitnesses))).toBe(EBallotStatus.WITNESSED)
    })

    it("shows late and problems over progress", () => {
        const late = resultOf("late", {record: "verified", witnesses: "verified", hubs: "late"})
        expect(ballotStatusOf(viewOf(late))).toBe(EBallotStatus.LATE)
        const failed = resultOf("failed", {record: "failed"})
        expect(ballotStatusOf(viewOf(failed))).toBe(EBallotStatus.PROBLEM)
        expect(ballotProblemOf(viewOf(failed))).toBe(EBallotProblem.MISMATCH)
        const invalid = resultOf("invalid", {record: "verified", witnesses: "invalid"})
        expect(ballotProblemOf(viewOf(invalid))).toBe(EBallotProblem.DAMAGED)
    })

    it("shows a problem when the receipt is missing or nothing could be checked", () => {
        const missing = viewOf(undefined, EReceiptNotice.MISSING)
        expect(ballotStatusOf(missing)).toBe(EBallotStatus.PROBLEM)
        expect(ballotProblemOf(missing)).toBe(EBallotProblem.NO_RECEIPT)
        const unchecked = viewOf(undefined, EReceiptNotice.UNCHECKED)
        expect(ballotProblemOf(unchecked)).toBe(EBallotProblem.UNCHECKED)
        expect(ballotProblemOf(viewOf(resultOf("verified", {})))).toBeUndefined()
    })
})

describe("statusSteps", () => {
    it("lists what each step will check before the ballot is cast", () => {
        expect(Object.values(statusSteps(EBallotStatus.NOT_CAST))).toEqual([
            EStepState.UPCOMING,
            EStepState.UPCOMING,
            EStepState.UPCOMING,
        ])
    })

    it("checks the record first, without a result", () => {
        expect(statusSteps(EBallotStatus.CHECKING)).toEqual({
            record: EStepState.ACTIVE,
            witnesses: EStepState.WAITING,
            anchors: EStepState.WAITING,
        })
        expect(statusSteps(EBallotStatus.PROBLEM)).toEqual({
            record: EStepState.FAILED,
            witnesses: EStepState.SKIPPED,
            anchors: EStepState.SKIPPED,
        })
    })

    it("makes each step wait for the ones before it", () => {
        const result = resultOf("pending", {
            record: "pending",
            witnesses: "verified",
            hubs: "verified",
        })
        expect(statusSteps(EBallotStatus.CHECKING, result)).toEqual({
            record: EStepState.ACTIVE,
            witnesses: EStepState.WAITING,
            anchors: EStepState.WAITING,
        })
    })

    it("joins the hubs and the browser layer into the anchors", () => {
        const late = resultOf("late", {
            record: "verified",
            witnesses: "verified",
            hubs: "verified",
            browser: "late",
        })
        expect(statusSteps(EBallotStatus.LATE, late).anchors).toBe(EStepState.LATE)
        const active = resultOf("pending", {
            record: "verified",
            witnesses: "verified",
            hubs: "verified",
            browser: "pending",
        })
        expect(statusSteps(EBallotStatus.WITNESSED, active).anchors).toBe(EStepState.ACTIVE)
    })

    it("skips the steps after one that failed", () => {
        const failed = resultOf("failed", {
            record: "verified",
            witnesses: "failed",
            hubs: "verified",
        })
        expect(statusSteps(EBallotStatus.PROBLEM, failed)).toEqual({
            record: EStepState.DONE,
            witnesses: EStepState.FAILED,
            anchors: EStepState.SKIPPED,
        })
    })
})

describe("the audit states", () => {
    const AUDIT_STATES = [EBallotStatus.AUDITABLE, EBallotStatus.AUDIT_FAILED]

    it("tells them from the states of a cast ballot", () => {
        expect(AUDIT_STATES.map(isAuditStatus)).toEqual([true, true])
        const others = Object.values(EBallotStatus).filter((s) => !AUDIT_STATES.includes(s))
        expect(others.some(isAuditStatus)).toBe(false)
    })

    it.each(AUDIT_STATES)("lists every step of an %s ballot as done", (status) => {
        const done = {
            record: EStepState.DONE,
            witnesses: EStepState.DONE,
            anchors: EStepState.DONE,
        }
        expect(statusSteps(status)).toEqual(done)
        const unfinished = resultOf("pending", PENDING)
        expect(statusSteps(status, unfinished)).toEqual(done)
    })

    it("never takes one from a receipt", () => {
        const verified = resultOf("verified", {
            record: "verified",
            witnesses: "verified",
            hubs: "verified",
            browser: "verified",
        })
        const views = [
            undefined,
            viewOf(),
            viewOf(resultOf("failed", {record: "failed"})),
            viewOf(verified),
            viewOf(undefined, EReceiptNotice.MISSING),
        ]
        expect(views.map((view) => isAuditStatus(ballotStatusOf(view)))).toEqual(
            views.map(() => false)
        )
    })
})

describe("ballotTagOf", () => {
    it("tags a ballot that is not cast, one that is cast in any state, and one that is audited", () => {
        expect(ballotTagOf(EBallotStatus.NOT_CAST)).toBe(EBallotTag.NOT_CAST)
        const cast = [
            EBallotStatus.CHECKING,
            EBallotStatus.RECORDED,
            EBallotStatus.WITNESSED,
            EBallotStatus.VERIFIED,
            EBallotStatus.LATE,
            EBallotStatus.PROBLEM,
        ]
        expect(cast.map(ballotTagOf)).toEqual(cast.map(() => EBallotTag.CAST))
        expect(ballotTagOf(EBallotStatus.AUDITABLE)).toBe(EBallotTag.AUDITABLE)
        expect(ballotTagOf(EBallotStatus.AUDIT_FAILED)).toBe(EBallotTag.AUDITABLE)
    })

    it("uses the keys of the tag's words as its values", () => {
        expect(Object.values(EBallotTag)).toEqual(["notCast", "cast", "auditable"])
    })
})
