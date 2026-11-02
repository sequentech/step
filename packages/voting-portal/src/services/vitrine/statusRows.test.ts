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
        policy: {id: "x", event: "e", witnesses: [], hubs: [], ...policy},
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

    describe("with one hub", () => {
        const hub = {name: "hub-a", operator: "partner", bootstrap: false}
        const policy = {operator: "sequent", hubs: new Map([["hub-a", hub]])} as unknown as Policy
        const detail = (platform?: string) =>
            statusRows({
                status: EBallotStatus.VERIFIED,
                result: resultOf(ALL_VERIFIED, {hubs: ["hub-a"], browser: "waict"}),
                policy,
                ...(platform ? {platform} : {}),
            })[2].detail

        it("says it is anchored in 1 hub, and who runs it", () => {
            expect(detail()).toEqual({
                key: "vitrine.status.row.anchoredOneHub",
                values: {party: "partner"},
            })
        })

        it("says the platform runs it when the policy names the platform as its operator", () => {
            expect(detail("partner")).toEqual({
                key: "vitrine.status.row.anchoredOneHubPlatform",
                values: {party: "partner"},
            })
        })
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

describe("the audit states", () => {
    it.each([
        [EBallotStatus.AUDITABLE, "auditable"],
        [EBallotStatus.AUDIT_FAILED, "auditfail"],
    ] as const)("says what %s means", (status, word) => {
        expect(statusHeadline({status})).toEqual({
            head: {key: `vitrine.status.state.${word}`},
            sub: {key: `vitrine.status.sub.${word}`},
        })
    })

    it.each([EBallotStatus.AUDITABLE, EBallotStatus.AUDIT_FAILED])(
        "lists all three steps of an %s ballot as done, whatever its receipt says",
        (status) => {
            const unfinished = resultOf([
                tier("record", "pending"),
                tier("witnesses", "pending"),
                tier("hubs", "pending"),
                tier("browser", "pending"),
            ])
            for (const result of [undefined, unfinished]) {
                const rows = statusRows({status, result})
                expect(rows.map((r) => r.state)).toEqual(["done", "done", "done"])
                expect(rows.map((r) => r.status)).toEqual([undefined, undefined, undefined])
            }
        }
    )

    it("uses the receipt's facts where it has them", () => {
        const rows = statusRows({
            status: EBallotStatus.AUDITABLE,
            result: resultOf(ALL_VERIFIED, {witnesses: [{name: "a"}], hubs: ["h1", "h2"]}),
        })
        expect(rows.map((r) => r.detail)).toEqual([
            {key: "vitrine.status.row.recordedEntry", values: {index: 7}},
            {key: "vitrine.status.row.witnessed", values: {count: 1}},
            {key: "vitrine.status.row.anchoredHubs", values: {count: 2}},
        ])
    })
})

describe("the caption", () => {
    it("says an audited ballot was decrypted, with no receipt to keep", () => {
        expect(statusCaption(EBallotStatus.AUDITABLE).map((t) => t.key)).toEqual([
            "vitrine.status.caption.auditable",
        ])
        expect(statusCaption(EBallotStatus.AUDIT_FAILED).map((t) => t.key)).toEqual([
            "vitrine.status.caption.problem",
        ])
    })

    it("asks the voter to keep the receipt to check again later, once it is checked", () => {
        const keys = (status: EBallotStatus) =>
            statusCaption(status).map((t) => t.key.split(".").pop())
        expect(keys(EBallotStatus.VERIFIED)).toEqual(["verified", "keep"])
        expect(keys(EBallotStatus.WITNESSED)).toEqual(["checked", "keep"])
        expect(keys(EBallotStatus.LATE)).toEqual(["checked", "keep"])
        expect(keys(EBallotStatus.NOT_CAST)).toEqual(["notcast"])
        expect(keys(EBallotStatus.CHECKING)).toEqual(["checking"])
        expect(keys(EBallotStatus.PROBLEM)).toEqual(["problem"])
    })
})
