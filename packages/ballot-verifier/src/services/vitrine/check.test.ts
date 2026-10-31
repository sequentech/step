// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import type {ChainResult} from "@vitrine/verify"
import {
    BALLOT_ID,
    TRACKING_ID,
    ballotEntry,
    ELECTION_ID,
    EVENT_ID,
    FakeNet,
    LOG_URLS,
    MIRROR_URL,
    MONITOR_URL,
    NOW,
    receiptScenario,
    referenceOf,
    referenceReceiptUrl,
    WITNESSES,
} from "../../fixtures/vitrine"
import {groupBallotId} from "./ballotId"
import {
    chainFromReference,
    checkRound,
    ECheckRound,
    expectedBallotId,
    expectFromForm,
    fullRoundDelay,
    logUrls,
    MAX_FULL_ROUND_DELAY_MS,
    MIN_FULL_ROUND_DELAY_MS,
    needsFullRoundNow,
} from "./check"
import {examinePolicy, IPolicyView} from "./policies"
import {ETransparencyMode} from "./presentation"
import type {IVitrineSdk} from "./sdk"

const sdk: IVitrineSdk = {verify, pure}

beforeAll(async () => {
    await verify.init()
})

const viewOf = (text: string): IPolicyView => {
    const examined = examinePolicy(sdk, text)
    if (!examined.ok) throw new Error("not a valid policy")
    return examined.view
}

const underLogs = (url: string) =>
    url.startsWith(LOG_URLS.ballots) || url.startsWith(LOG_URLS.events)

describe("fullRoundDelay", () => {
    it("falls between one and thirty minutes", () => {
        expect(fullRoundDelay(() => 0)).toBe(MIN_FULL_ROUND_DELAY_MS)
        expect(fullRoundDelay(() => 0.5)).toBe(
            (MIN_FULL_ROUND_DELAY_MS + MAX_FULL_ROUND_DELAY_MS) / 2
        )
        expect(fullRoundDelay(() => 0.999999)).toBeLessThan(MAX_FULL_ROUND_DELAY_MS)
    })
})

describe("needsFullRoundNow", () => {
    const resultWith = (tiers: unknown[]) => ({tiers}) as unknown as ChainResult

    it("can't wait when a hub or the browser layer is late, or a hub is behind", () => {
        expect(needsFullRoundNow(resultWith([{tier: "hubs", status: "late", reasons: []}]))).toBe(
            true
        )
        expect(
            needsFullRoundNow(resultWith([{tier: "browser", status: "late", reasons: []}]))
        ).toBe(true)
        expect(
            needsFullRoundNow(
                resultWith([
                    {
                        tier: "hubs",
                        status: "pending",
                        reasons: [],
                        hubs: [{status: "verified", reasons: [{code: "anchor-behind-record"}]}],
                    },
                ])
            )
        ).toBe(true)
    })

    it("waits otherwise", () => {
        expect(
            needsFullRoundNow(
                resultWith([
                    {tier: "record", status: "late", reasons: []},
                    {tier: "hubs", status: "pending", reasons: [], hubs: []},
                ])
            )
        ).toBe(false)
    })
})

describe("expectFromForm", () => {
    it("expects the entry of that tracking ID, election and event", () => {
        expect(
            expectFromForm(sdk, {
                eventId: ` ${EVENT_ID} `,
                electionId: ELECTION_ID,
                ballotId: ` ${TRACKING_ID.toLowerCase()}\n`,
                stage: "cast",
            })
        ).toEqual({fields: {event_id: EVENT_ID, election_id: ELECTION_ID, ballot_id: BALLOT_ID}})
    })

    it("reads a tracking ID in groups of four, as the field shows it", () => {
        expect(
            expectFromForm(sdk, {
                eventId: EVENT_ID,
                electionId: ELECTION_ID,
                ballotId: groupBallotId(TRACKING_ID),
                stage: "cast",
            })
        ).toEqual({fields: {event_id: EVENT_ID, election_id: ELECTION_ID, ballot_id: BALLOT_ID}})
    })

    it.each([
        ["a short tracking ID", {ballotId: "FTBE-MHRX"}],
        ["a voter pseudonym", {ballotId: "FTBE:MHRX"}],
        ["a tracking ID that isn't Crockford base32", {ballotId: "<script>alert(1)</script>"}],
        ["no election", {electionId: ""}],
        ["no event", {eventId: ""}],
    ])("refuses %s", (_what, change) => {
        const form = {eventId: EVENT_ID, electionId: ELECTION_ID, ballotId: TRACKING_ID, stage: "cast" as const, ...change}
        expect(expectFromForm(sdk, form)).toBeUndefined()
    })
})

describe("expectedBallotId", () => {
    it("reads the tracking ID from the fields or from the entry", () => {
        const expect_ = expectFromForm(sdk, {
            eventId: EVENT_ID,
            electionId: ELECTION_ID,
            ballotId: TRACKING_ID,
            stage: "cast",
        })
        expect(expect_ && expectedBallotId(sdk, expect_)).toBe(TRACKING_ID)
        expect(expectedBallotId(sdk, {entry: ballotEntry()})).toBe(TRACKING_ID)
        expect(expectedBallotId(sdk, {entry: Uint8Array.of(1, 2, 3)})).toBeUndefined()
    })
})

describe("checkRound", () => {
    const s = receiptScenario()
    const view = viewOf(s.policy)

    const run = async (round: ECheckRound.LOG | ECheckRound.FULL, v: IPolicyView = view) => {
        const net = new FakeNet()
        net.serve(`${LOG_URLS.ballots}checkpoint`, s.ballots.checkpoint(2, WITNESSES))
        const outcome = await checkRound(
            sdk,
            {
                chain: s.chain,
                view: v,
                expect: {entry: s.entry},
                history: new verify.PolicyHistory(v.policy),
            },
            round,
            {fetch: net.fetch, now: () => NOW}
        )
        return {net, outcome}
    }

    it("asks only the policy's logs in the first round", async () => {
        const {net, outcome} = await run(ECheckRound.LOG)
        expect(outcome.round).toBe(ECheckRound.LOG)
        expect(outcome.result.where).toBe("independent")
        expect(outcome.result.ok).toBe(true)
        expect(net.calls.length).toBeGreaterThan(0)
        expect(net.calls.filter((c) => !underLogs(c.url))).toEqual([])
        expect(net.calls.every((c) => c.method === "GET")).toBe(true)
        expect(logUrls(view)).toEqual([LOG_URLS.ballots, LOG_URLS.events])
    })

    it("also asks a mirror and the monitors in the full round", async () => {
        const {net, outcome} = await run(ECheckRound.FULL)
        expect(outcome.round).toBe(ECheckRound.FULL)
        expect(net.requests()).toContain(`GET ${MIRROR_URL}checkpoint`)
        expect(net.requests()).toContain(`POST ${MONITOR_URL}gossip`)
    })

    it("checks a private event's receipt with what the chain holds", async () => {
        const p = receiptScenario({visibility: ETransparencyMode.PRIVATE})
        const v = viewOf(p.policy)
        const net = new FakeNet()
        const outcome = await checkRound(
            sdk,
            {
                chain: p.chain,
                view: v,
                expect: {entry: p.entry},
                history: new verify.PolicyHistory(v.policy),
            },
            ECheckRound.FULL,
            {fetch: net.fetch, now: () => NOW}
        )
        expect(outcome.round).toBe(ECheckRound.OFFLINE)
        expect(outcome.chain).toBe(p.chain)
        expect(outcome.result.where).toBe("independent")
        expect(net.calls).toEqual([])
    })
})

describe("chainFromReference", () => {
    it("rebuilds the chain from the log alone", async () => {
        const s = receiptScenario()
        const view = viewOf(s.policy)
        const net = new FakeNet()
        net.serve(referenceReceiptUrl(s), s.receipt)
        const chain = await chainFromReference(
            sdk,
            verify.parseReceiptReference(referenceOf(s)),
            view,
            new verify.PolicyHistory(view.policy),
            {fetch: net.fetch, now: () => NOW}
        )
        const issued = pure.parseChain(chain).issued
        expect(issued && pure.bytes.fromUtf8(issued)).toBe(s.receipt)
        expect(net.requests()).toEqual([`GET ${referenceReceiptUrl(s)}`])
    })
})
