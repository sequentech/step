// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    ACCEPTED_AT,
    ballotEntry,
    chainText,
    IScenario,
    IScenarioOptions,
    KEYS,
    LOG_URLS,
    MONITOR_URL,
    NOW,
    EVENT_ID,
    ELECTION_ID,
    scenario,
} from "../../fixtures/vitrine"
import {
    ANCHORS_WAIT,
    EReceiptNotice,
    ICastReceiptOptions,
    IReceiptView,
    MAX_POLL_MS,
    runCastReceipt,
    runTrackedReceipt,
    TForward,
} from "./castReceipt"
import type {THttpInit} from "./http"
import {readTransparency} from "./presentation"
import type {IVitrineSdk} from "./sdk"
import {ETransparencyMode, EVitrineForwardRoute, IVitrineCast} from "./types"

const sdk: IVitrineSdk = {verify, pure}
const WITNESSES = KEYS.witnesses.slice(0, 2)

beforeAll(async () => {
    await verify.init()
})

interface IFakeAnswer {
    status: number
    body?: string
}

interface ICall {
    url: string
    method: string
    body?: string
}

const response = (a: IFakeAnswer) =>
    ({
        status: a.status,
        headers: {get: () => null},
        body: null,
        arrayBuffer: async () => pure.bytes.utf8(a.body ?? ""),
    }) as unknown as Response

/** The network: routes answer what they match, POSTs get 202 and the rest 404. */
class FakeNet {
    readonly calls: ICall[] = []
    readonly routes: ((call: ICall) => IFakeAnswer | undefined)[] = []

    readonly fetch = jest.fn(async (input: string, init?: THttpInit) => {
        const call: ICall = {
            url: String(input),
            method: init?.method ?? "GET",
            ...(typeof init?.body === "string" ? {body: init.body} : {}),
        }
        this.calls.push(call)
        for (const route of this.routes) {
            const answer = route(call)
            if (answer) return response(answer)
        }
        return response({status: call.method === "POST" ? 202 : 404})
    })

    posts(path: string): ICall[] {
        return this.calls.filter((c) => c.method === "POST" && c.url === `${MONITOR_URL}${path}`)
    }
}

// Each test casts its own ballot: observations go once per ballot and page load.
let ballots = 0
const nextBallotId = (): string => (++ballots).toString(16).padStart(64, "0")

interface IRun {
    net: FakeNet
    views: IReceiptView[]
    last?: IReceiptView
}

const cast = async (
    s: IScenario,
    ballotId: string,
    answer: IVitrineCast | null,
    extra: Partial<ICastReceiptOptions> & {net?: FakeNet} = {}
): Promise<IRun> => {
    const net = extra.net ?? new FakeNet()
    let t = NOW * 1000
    const views: IReceiptView[] = []
    await runCastReceipt({
        cast: answer,
        target: {eventId: EVENT_ID, electionId: ELECTION_ID, stage: "cast", ballotId},
        transparency: readTransparency(s.presentation),
        context: async () => s.context,
        fetch: net.fetch as unknown as typeof fetch,
        now: () => t,
        sleep: async (ms) => {
            t += ms
        },
        report: {
            fetch: net.fetch,
            schedule: (send) => {
                void send()
            },
            random: () => 0,
        },
        loadSdk: async () => sdk,
        onView: (view) => views.push(view),
        ...extra,
    })
    return {net, views, last: views[views.length - 1]}
}

const parsed = (chain: string | undefined) => pure.parseChain(chain ?? "")
const text = (b: Uint8Array | undefined) => (b ? pure.bytes.fromUtf8(b) : undefined)

/** A log with the voter's entry and another, and a receipt for the first. */
const withReceipt = (options: IScenarioOptions = {}) => {
    const s = scenario(options)
    const ballotId = nextBallotId()
    const index = s.ballots.add(ballotEntry(ballotId))
    s.ballots.add(ballotEntry(nextBallotId()))
    return {s, ballotId, index, receipt: s.ballots.receipt(index, 2, WITNESSES)}
}

const observations = (net: FakeNet): string[] => net.posts("observe").map((c) => c.body ?? "")

describe("runCastReceipt", () => {
    it("shows a receipt for the voter's own entry", async () => {
        const {s, ballotId, receipt} = withReceipt()
        const {net, last} = await cast(s, ballotId, {kind: "receipt", text: receipt})
        expect(last?.notice).toBe(EReceiptNotice.NONE)
        expect(text(parsed(last?.chain).issued)).toBe(receipt)
        // The fake log serves no tiles, so the record waits for its successors.
        expect(last?.result?.ok).toBe(true)
        const record = last?.result?.tiers.find((t) => t.tier === "record")
        expect(record?.status).toBe("pending")
        expect(record?.reasons.map((r) => r.code)).toEqual(["successors-unchecked"])
        expect(last?.own?.acceptedAt).toBe(ACCEPTED_AT)
        expect(observations(net)).toEqual([])
    })

    it("reads when the witnesses signed from the receipt, for the lists of who signed", async () => {
        const {s, ballotId, receipt} = withReceipt()
        const {last} = await cast(s, ballotId, {kind: "receipt", text: receipt})
        expect(last?.signers?.witnesses).toEqual(
            new Map(WITNESSES.map((_, n) => [`w${n + 1}`, NOW - 10]))
        )
        expect(last?.signers?.size).toBe(2)
    })

    it("keeps asking the log for the anchors while the screen is open, until they're due", async () => {
        const {s, ballotId, receipt} = withReceipt()
        const checkChain = jest.fn(verify.checkChain)
        let t = NOW * 1000
        const {last} = await cast(
            s,
            ballotId,
            {kind: "receipt", text: receipt},
            {
                loadSdk: async () => ({verify: {...verify, checkChain}, pure}),
                now: () => t,
                sleep: async (ms) => {
                    t += ms
                },
            }
        )
        expect(last?.result?.tiers.find((tier) => tier.tier === "hubs")?.status).toBe("pending")
        expect(checkChain.mock.calls.length).toBeGreaterThan(1)
        expect(t / 1000).toBeLessThanOrEqual(NOW + ANCHORS_WAIT + MAX_POLL_MS / 1000)
    })

    it("reports a receipt for another ballot, without the receipt or the entry", async () => {
        const {s, receipt} = withReceipt()
        const ballotId = nextBallotId()
        const {net, last} = await cast(s, ballotId, {kind: "receipt", text: receipt})
        expect(last?.notice).toBe(EReceiptNotice.MISSING)
        expect(last?.chain).toBeUndefined()
        const sent = observations(net)
        expect(sent).toHaveLength(1)
        expect(sent[0]).toContain("kind no-receipt")
        expect(sent[0]).toContain("log ballots")
        expect(sent[0]).toContain("answer receipt entry-mismatch")
        expect(sent[0]).not.toContain(receipt.split("\n")[0])
        expect(sent[0]).not.toContain(ballotId)
    })

    it("treats text as data: markup is an invalid receipt", async () => {
        const s = scenario()
        const {net, last, views} = await cast(s, nextBallotId(), {
            kind: "receipt",
            text: "<img src=x onerror=alert(1)>",
        })
        expect(last?.notice).toBe(EReceiptNotice.MISSING)
        expect(JSON.stringify(views)).not.toContain("onerror")
        expect(observations(net)[0]).toContain("answer receipt invalid")
    })

    it("accepts a promise near now, escrows it and keeps asking for its receipt", async () => {
        const s = scenario()
        const ballotId = nextBallotId()
        const index = s.ballots.add(ballotEntry(ballotId))
        const promise = s.ballots.promise(index, [index], s.policyId, NOW - 2)
        const {net, last} = await cast(s, ballotId, {kind: "promise", text: promise})
        expect(last?.notice).toBe(EReceiptNotice.NONE)
        expect(text(parsed(last?.chain).promise)).toBe(promise)
        expect(net.posts("promise").map((c) => c.body)).toEqual([promise])
        expect(observations(net)).toEqual([])
        const asked = net.calls.filter((c) => c.url.includes("/receipt/"))
        expect(asked.length).toBeGreaterThan(1)
        expect(asked.every((c) => c.url.startsWith(LOG_URLS.ballots))).toBe(true)
    })

    it("refuses a promise dated away from now, but still escrows it", async () => {
        const s = scenario()
        const ballotId = nextBallotId()
        const index = s.ballots.add(ballotEntry(ballotId, NOW - 125))
        const promise = s.ballots.promise(index, [index], s.policyId, NOW - 120)
        const {net, last} = await cast(s, ballotId, {kind: "promise", text: promise})
        expect(last?.notice).toBe(EReceiptNotice.MISSING)
        expect(net.posts("promise")).toHaveLength(1)
        expect(observations(net)[0]).toContain("answer promise promise-time")
    })

    it("asks the log for a queued entry's receipt and shows it when it comes", async () => {
        const s = scenario()
        const ballotId = nextBallotId()
        const index = s.ballots.add(ballotEntry(ballotId))
        s.ballots.add(ballotEntry(nextBallotId()))
        const receipt = s.ballots.receipt(index, 2, WITNESSES)
        const leaf = pure.encoding.hexEncode(pure.hashes.leafHash(s.ballots.entries[index]))
        const net = new FakeNet()
        let asked = 0
        net.routes.push((c) =>
            c.url === `${LOG_URLS.ballots}receipt/${leaf}?near=${ACCEPTED_AT}` && ++asked > 1
                ? {status: 200, body: receipt}
                : undefined
        )
        const entry = pure.encoding.b64encode(s.ballots.entries[index])
        const {views, last} = await cast(
            s,
            ballotId,
            {kind: "queued", text: entry, reason: "busy"},
            {net}
        )
        expect(views[0].notice).toBe(EReceiptNotice.MISSING)
        expect(observations(net)[0]).toContain("answer queued busy")
        expect(last?.notice).toBe(EReceiptNotice.NONE)
        expect(text(parsed(last?.chain).issued)).toBe(receipt)
    })

    it.each([
        [{kind: "unavailable", reason: "log-down"}, "answer unavailable log-down"],
        [{kind: "<b>receipt</b>", text: "x"}, "answer none unknown-kind"],
        [null, "answer none no-vitrine"],
        [{kind: "receipt", text: ""}, "answer receipt empty"],
        [{kind: "queued", reason: "Not A Code!"}, "answer queued\n"],
    ])("reports %j as %s", async (answer, line) => {
        const {net, last} = await cast(scenario(), nextBallotId(), answer)
        expect(last?.notice).toBe(EReceiptNotice.MISSING)
        expect(observations(net)[0]).toContain(line)
    })

    it("reports once per ballot and page load", async () => {
        const s = scenario()
        const ballotId = nextBallotId()
        const first = await cast(s, ballotId, {kind: "unavailable"})
        const again = await cast(s, ballotId, {kind: "unavailable"})
        expect(observations(first.net)).toHaveLength(1)
        expect(observations(again.net)).toEqual([])
        expect(again.last?.notice).toBe(EReceiptNotice.MISSING)
    })

    it("waits without a failure or a report under `required off`", async () => {
        const s = scenario({policyExtra: ["required off"]})
        const ballotId = nextBallotId()
        const entry = pure.encoding.b64encode(ballotEntry(ballotId))
        const {net, last} = await cast(s, ballotId, {kind: "queued", text: entry})
        expect(last?.notice).toBe(EReceiptNotice.WAITING)
        expect(observations(net)).toEqual([])
    })

    it("shows nothing it could check without a pinned policy", async () => {
        const s = scenario()
        const presentation = {...s.presentation, vitrine: {policy_id: "00".repeat(32)}}
        const {last} = await cast({...s, presentation}, nextBallotId(), {kind: "unavailable"})
        expect(last).toEqual({notice: EReceiptNotice.UNCHECKED})
    })

    it("does nothing for events without transparency", async () => {
        const {views, net} = await cast({...scenario(), presentation: {}}, nextBallotId(), {
            kind: "unavailable",
        })
        expect(views).toEqual([])
        expect(net.calls).toEqual([])
    })

    describe("private events", () => {
        const privately = () => {
            const s = scenario({mode: ETransparencyMode.PRIVATE})
            const ballotId = nextBallotId()
            const index = s.ballots.add(ballotEntry(ballotId))
            return {s, ballotId, index}
        }

        it.each([true, false])(
            "keeps selective private discovery pending even with an authentic checkpoint (%s)",
            async (valid) => {
                const {s, ballotId, index} = privately()
                const receipt = s.ballots.receipt(index, 1, WITNESSES)
                const proof = pure.bytes.fromUtf8(
                    pure.history.formatPolicyProofs({
                        log: "events",
                        checkpoint: pure.bytes.utf8(
                            s.events.checkpoint(s.events.entries.length, WITNESSES, NOW)
                        ),
                        entries: [],
                    })
                )
                const forward = jest.fn<ReturnType<TForward>, Parameters<TForward>>(
                    async (route) =>
                        route === EVitrineForwardRoute.POLICIES
                            ? {status: 200, body: valid ? proof : "not policy proofs"}
                            : {status: 404}
                )
                const {last, net} = await cast(
                    s,
                    ballotId,
                    {kind: "receipt", text: receipt},
                    {forward}
                )
                expect(forward).toHaveBeenNthCalledWith(1, EVitrineForwardRoute.POLICIES, "")
                expect(last?.result?.tiers[0].status).toBe("pending")
                expect(
                    last?.result?.tiers.some((tier) =>
                        tier.reasons.some((r) => r.code === "successors-unchecked")
                    )
                ).toBe(true)
                expect(
                    net.calls.filter(
                        (c) =>
                            c.url.startsWith(LOG_URLS.ballots) || c.url.startsWith(LOG_URLS.events)
                    )
                ).toEqual([])
            }
        )

        it("keeps the policy acknowledgement and merges the upgrade Harvest forwards", async () => {
            const {s, ballotId, index} = privately()
            const issued = s.ballots.receipt(index, 1, WITNESSES)
            s.ballots.add(ballotEntry(nextBallotId()))
            const record = s.ballots.receipt(index, 2, WITNESSES)
            const upgrade = chainText(s.policyId, "ballots", {
                record: pure.bytes.utf8(record),
                consistency: pure.consistencyBlock(
                    BigInt(1),
                    BigInt(2),
                    s.ballots.consistency(1, 2)
                ),
            })
            const forward = jest.fn<ReturnType<TForward>, Parameters<TForward>>(async (route) =>
                route === EVitrineForwardRoute.UPGRADE
                    ? {status: 200, body: upgrade}
                    : {status: 404}
            )
            const {net, last} = await cast(s, ballotId, {kind: "receipt", text: issued}, {forward})
            expect(forward).toHaveBeenCalledWith(EVitrineForwardRoute.UPGRADE, issued)
            const chain = parsed(last?.chain)
            expect(text(chain.policyAck)).toBe(s.policyAck)
            expect(text(chain.issued)).toBe(issued)
            expect(text(chain.record)).toBe(record)
            expect(chain.consistency?.to).toBe(BigInt(2))
            expect(net.calls.filter((c) => c.url.startsWith(LOG_URLS.ballots))).toEqual([])
        })

        it("keeps a promise while the log answers 425 in a strict window", async () => {
            const {s, ballotId, index} = privately()
            const promise = s.ballots.promise(index, [index], s.policyId, NOW - 2)
            const forward = jest.fn<ReturnType<TForward>, Parameters<TForward>>(async () => ({
                status: 425,
            }))
            const {last} = await cast(s, ballotId, {kind: "promise", text: promise}, {forward})
            expect(forward).toHaveBeenCalledWith(EVitrineForwardRoute.UPGRADE, promise)
            expect(last?.notice).toBe(EReceiptNotice.NONE)
            expect(text(parsed(last?.chain).promise)).toBe(promise)
            expect(parsed(last?.chain).record).toBeUndefined()
        })
    })
})

describe("runTrackedReceipt", () => {
    it("asks the log once for the receipt of an entry the tracker found", async () => {
        const {s, ballotId, index, receipt} = withReceipt()
        const leaf = pure.encoding.hexEncode(pure.hashes.leafHash(s.ballots.entries[index]))
        const net = new FakeNet()
        net.routes.push((c) =>
            c.url.startsWith(`${LOG_URLS.ballots}receipt/${leaf}`)
                ? {status: 200, body: receipt}
                : undefined
        )
        const views: IReceiptView[] = []
        await runTrackedReceipt({
            acceptedAt: ACCEPTED_AT,
            target: {eventId: EVENT_ID, electionId: ELECTION_ID, stage: "cast", ballotId},
            transparency: readTransparency(s.presentation),
            fetch: net.fetch as unknown as typeof fetch,
            now: () => NOW * 1000,
            loadSdk: async () => sdk,
            onView: (view) => views.push(view),
        })
        expect(views[0].notice).toBe(EReceiptNotice.WAITING)
        const last = views[views.length - 1]
        expect(last.notice).toBe(EReceiptNotice.NONE)
        expect(text(parsed(last.chain).issued)).toBe(receipt)
        expect(net.calls.filter((c) => c.url.includes("/receipt/"))).toHaveLength(1)
        expect(observations(net)).toEqual([])
    })
})
