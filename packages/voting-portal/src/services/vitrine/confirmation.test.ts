// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EReceiptNotice} from "./castReceipt"
import {
    confirmationPhase,
    EConfirmationPhase,
    ERevisitPhase,
    failureOf,
    receiptFileName,
    revisitSummary,
} from "./confirmation"

const tier = (name: TierName, status: TierStatus, extra: Partial<TierResult> = {}): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...extra,
})

const resultOf = (
    status: ChainResult["status"],
    tiers: TierResult[],
    unchanged: ChainResult["facts"]["unchanged"] = "pending"
): ChainResult =>
    ({
        ok: status !== "failed" && status !== "invalid",
        status,
        tiers,
        facts: {included: "done", unchanged, sameForEveryone: "pending"},
        reasons: [],
        steps: [],
        where: "page",
        entry: {log: "ballots", leafHash: "ab", index: 18, promised: false},
        words: [],
        wordsLang: "en",
        now: 0,
        clockWarning: false,
        profileMetadataResolved: true,
        policy: {id: "x", words: [], lang: "en", event: "e", witnesses: [], hubs: []},
    }) as unknown as ChainResult

const PENDING = [
    tier("record", "verified"),
    tier("witnesses", "pending"),
    tier("hubs", "pending"),
    tier("browser", "pending"),
]
const WITNESSED = [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "pending"),
    tier("browser", "pending"),
]

describe("confirmationPhase", () => {
    it("says the ballot is cast until the witnesses confirm it", () => {
        expect(confirmationPhase(EReceiptNotice.NONE, resultOf("pending", PENDING))).toBe(
            EConfirmationPhase.CAST
        )
        expect(confirmationPhase(EReceiptNotice.WAITING, undefined)).toBe(EConfirmationPhase.CAST)
        expect(confirmationPhase(EReceiptNotice.UNCHECKED, undefined)).toBe(EConfirmationPhase.CAST)
    })

    it("says the ballot is in the record once the record and the witnesses are verified", () => {
        expect(confirmationPhase(EReceiptNotice.NONE, resultOf("pending", WITNESSED))).toBe(
            EConfirmationPhase.RECORDED
        )
    })

    it("fails on contradicting proofs, or on the receipt a policy required and never got", () => {
        const failed = [tier("record", "verified"), tier("witnesses", "failed")]
        expect(confirmationPhase(EReceiptNotice.NONE, resultOf("failed", failed))).toBe(
            EConfirmationPhase.FAILED
        )
        expect(confirmationPhase(EReceiptNotice.MISSING, undefined)).toBe(EConfirmationPhase.FAILED)
    })

    it("doesn't read damaged data as a failure", () => {
        const invalid = [tier("record", "invalid"), tier("witnesses", "pending")]
        expect(confirmationPhase(EReceiptNotice.NONE, resultOf("invalid", invalid))).toBe(
            EConfirmationPhase.CAST
        )
    })
})

describe("revisitSummary", () => {
    it("holds when the record is included, witnessed and unchanged", () => {
        const hubs = tier("hubs", "verified", {
            hubs: [
                {hub: "a", status: "verified", reasons: []},
                {hub: "b", status: "pending", reasons: []},
            ],
        })
        const summary = revisitSummary(
            resultOf(
                "pending",
                [tier("record", "verified"), tier("witnesses", "verified"), hubs],
                "done"
            )
        )
        expect(summary.phase).toBe(ERevisitPhase.HOLDS)
        expect(summary.witnesses).toBe("verified")
        expect(summary.hubs).toEqual({agree: 1, total: 2, status: "verified"})
    })

    it("waits for proofs that are still on their way", () => {
        expect(revisitSummary(resultOf("pending", WITNESSED)).phase).toBe(ERevisitPhase.WAITING)
    })

    it("reports a problem only for contradicting proofs", () => {
        const failed = [tier("record", "verified"), tier("hubs", "failed")]
        expect(revisitSummary(resultOf("failed", failed)).phase).toBe(ERevisitPhase.PROBLEM)
        expect(revisitSummary(resultOf("invalid", PENDING)).phase).toBe(ERevisitPhase.UNCHECKED)
        expect(revisitSummary(undefined).phase).toBe(ERevisitPhase.UNCHECKED)
    })
})

describe("failureOf", () => {
    it("lists the entry and the tiers whose proofs contradict each other", () => {
        const reason = {code: "split-view", objects: ["issued", "record"]}
        const failed = [
            tier("record", "verified"),
            tier("witnesses", "failed", {reasons: [reason]}),
        ]
        expect(failureOf(resultOf("failed", failed))).toEqual({
            entry: {log: "ballots", index: 18},
            problems: [{tier: "witnesses", reasons: [reason]}],
        })
    })

    it("shows no evidence for a result that isn't a failure", () => {
        expect(failureOf(resultOf("invalid", PENDING))).toBeUndefined()
        expect(failureOf(resultOf("verified", WITNESSED))).toBeUndefined()
        expect(failureOf(undefined)).toBeUndefined()
    })
})

describe("receiptFileName", () => {
    it("names the file after the ballot, safely", () => {
        expect(receiptFileName("abcd1234ef56")).toBe("receipt-abcd1234.vitrine-receipt")
        expect(receiptFileName("../../x")).toBe("receipt-x.vitrine-receipt")
        expect(receiptFileName("")).toBe("receipt-vitrine.vitrine-receipt")
    })
})
