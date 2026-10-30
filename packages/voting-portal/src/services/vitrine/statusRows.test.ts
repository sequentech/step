// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, Policy, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EBallotProblem, EBallotStatus} from "./ballotStatus"
import {anchorMinutes, statusCaption, statusHeadline, statusRows} from "./statusRows"

const tier = (name: TierName, status: TierStatus, extra: Partial<TierResult> = {}): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...extra,
})

const resultOf = (
    tiers: TierResult[],
    policy: Partial<ChainResult["policy"]> = {},
    entry: Partial<NonNullable<ChainResult["entry"]>> | null = {index: 7}
): ChainResult =>
    ({
        status: "verified",
        tiers,
        entry: entry ? {log: "ballots", leafHash: "ab", promised: false, ...entry} : undefined,
        policy: {id: "x", words: [], lang: "en", event: "e", witnesses: [], hubs: [], ...policy},
    }) as unknown as ChainResult

const ALL_VERIFIED = [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified"),
    tier("browser", "verified"),
]

const anchorsDetail = (policy: Partial<ChainResult["policy"]>) =>
    statusRows({
        status: EBallotStatus.VERIFIED,
        result: resultOf(ALL_VERIFIED, policy),
    })[2].detail

describe("anchorMinutes", () => {
    it("rounds the policy's browser period up to whole minutes", () => {
        expect(anchorMinutes({browserPeriod: 600} as Policy)).toBe(10)
        expect(anchorMinutes({browserPeriod: 90} as Policy)).toBe(2)
        expect(anchorMinutes({browserPeriod: 5} as Policy)).toBe(1)
    })

    it("falls back to ten minutes without a policy", () => {
        expect(anchorMinutes(undefined)).toBe(10)
    })
})

describe("the anchors step of a verified ballot", () => {
    it.each([
        ["waict", "row.anchoredWaict"],
        ["bootstrap", "row.anchoredBootstrap"],
        ["test", "row.anchoredHubs"],
        ["none", "row.anchoredHubs"],
    ] as const)("names the hubs, and the browser layer when it is %s", (browser, key) => {
        expect(anchorsDetail({hubs: ["a", "b"], browser})).toEqual({
            key: `vitrine.status.${key}`,
            values: {count: 2},
        })
    })

    it("says what the step is when the policy has no hubs", () => {
        expect(anchorsDetail({hubs: []}).key).toBe("vitrine.status.step.anchors.upcoming")
    })
})

describe("the other steps' facts", () => {
    it("names the entry only when the receipt gives its index", () => {
        const detail = (entry: {index?: number} | null) =>
            statusRows({
                status: EBallotStatus.VERIFIED,
                result: resultOf(ALL_VERIFIED, {}, entry),
            })[0].detail
        expect(detail({index: 7})).toEqual({
            key: "vitrine.status.row.recordedEntry",
            values: {index: 7},
        })
        expect(detail({})).toEqual({key: "vitrine.status.row.recorded"})
        expect(detail(null)).toEqual({key: "vitrine.status.row.recorded"})
    })

    it("counts the policy's witnesses when it names them", () => {
        const detail = (witnesses: {name: string}[]) =>
            statusRows({
                status: EBallotStatus.VERIFIED,
                result: resultOf(ALL_VERIFIED, {witnesses}),
            })[1].detail
        expect(detail([{name: "a"}, {name: "b"}])).toEqual({
            key: "vitrine.status.row.witnessed",
            values: {count: 2},
        })
        expect(detail([]).key).toBe("vitrine.status.step.witnesses.upcoming")
    })

    it("says a late step is late even when the receipt gives no deadline", () => {
        const late = resultOf([
            tier("record", "verified"),
            tier("witnesses", "verified"),
            tier("hubs", "late"),
            tier("browser", "pending"),
        ])
        const rows = statusRows({status: EBallotStatus.LATE, result: late})
        expect(rows[2].detail.key).toBe("vitrine.status.row.lateNow")
        expect(rows[2].status?.key).toBe("vitrine.status.word.late")
    })

    it("leaves a done step's time to the page that saw it", () => {
        const rows = statusRows({
            status: EBallotStatus.VERIFIED,
            result: resultOf(ALL_VERIFIED),
            times: {witnesses: 1234},
        })
        expect(rows.map((r) => r.doneAt)).toEqual([undefined, 1234, undefined])
    })
})

describe("the headline of a problem", () => {
    it("names the entry only when the receipt gives its index", () => {
        const sub = (entry: {index?: number} | null) =>
            statusHeadline({
                status: EBallotStatus.PROBLEM,
                problem: EBallotProblem.MISMATCH,
                result: resultOf([tier("record", "failed")], {}, entry),
            }).sub
        expect(sub({index: 21})).toEqual({key: "vitrine.status.sub.mismatch", values: {index: 21}})
        expect(sub(null).key).toBe("vitrine.status.sub.mismatchNoEntry")
    })
})

describe("the caption", () => {
    it("adds that this device keeps the receipt only when it does", () => {
        const keys = (status: EBallotStatus, kept: boolean) =>
            statusCaption(status, kept).map((t) => t.key.split(".").pop())
        expect(keys(EBallotStatus.VERIFIED, true)).toEqual(["verified", "kept"])
        expect(keys(EBallotStatus.WITNESSED, false)).toEqual(["checked"])
        expect(keys(EBallotStatus.NOT_CAST, true)).toEqual(["notcast"])
        expect(keys(EBallotStatus.PROBLEM, true)).toEqual(["problem"])
    })
})
