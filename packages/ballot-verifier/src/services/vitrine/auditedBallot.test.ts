// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import type {BallotMatch, Policy} from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    auditedScenario,
    AUDITED_AT,
    BALLOT_ID,
    ballotEntry,
    bucketAnswer,
    bucketUrlOf,
    ELECTION_ID,
    EVENT_ID,
    FakeNet,
    KEYS,
    LOG_SERVER,
    LOG_URLS,
    MONITOR_URL,
    OTHER_BALLOT_ID,
    OTHER_ELECTION_ID,
    OTHER_TRACKING_ID,
    policyText,
    TestLog,
    TRACKING_ID,
} from "../../fixtures/vitrine"
import {
    auditedBallotLink,
    auditedReference,
    ballotStage,
    bucketServers,
    EEntriesKind,
    ELocateKind,
    EPolicyProblem,
    EStageKind,
    findBallotEntries,
    hubIdentity,
    loadAuditedPolicy,
    locateBallot,
    parseAuditedFragment,
    publicConfigUrl,
} from "./auditedBallot"
import {examinePolicy, PINNED_POLICIES_KEY} from "./policies"
import type {IVitrineSdk} from "./sdk"

const sdk: IVitrineSdk = {verify, pure}
const TENANT_ID = "7c1e5d0a-2b3f-4c4d-8e5f-6a7b8c9d0e1f"
const IDS = {tenantId: TENANT_ID, eventId: EVENT_ID, electionId: ELECTION_ID}
const BUCKET = "https://s3.test/public/"
const GRAPHQL = "https://hasura.test/v1/graphql"
const POLICY_ID = `sha256:${"ab".repeat(32)}`
const CHARS = TRACKING_ID.replace(/-/g, "")

let net: FakeNet

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    localStorage.clear()
    net = new FakeNet()
})

const policyOf = (text: string): Policy => {
    const examined = examinePolicy(sdk, text)
    if (!examined.ok) throw new Error("the test policy doesn't parse")
    return examined.view.policy
}

const config = (policy: string, ids = IDS) =>
    JSON.stringify({
        tenant_id: ids.tenantId,
        election_event_id: ids.eventId,
        election_event_presentation: {vitrine: {policy}},
    })

describe("parseAuditedFragment", () => {
    it("reads the full Tracking ID and the policy ID", () => {
        expect(parseAuditedFragment(`#id=${TRACKING_ID}&policy=${POLICY_ID}`)).toEqual({
            trackingId: TRACKING_ID,
            policyId: POLICY_ID,
        })
        // Without its dashes, in lower case, and a policy ID in upper case.
        expect(
            parseAuditedFragment(`id=${CHARS.toLowerCase()}&policy=${POLICY_ID.toUpperCase()}`)
        ).toEqual({trackingId: TRACKING_ID, policyId: POLICY_ID})
    })

    it.each([
        ["no fragment", ""],
        ["no policy", `#id=${TRACKING_ID}`],
        ["no Tracking ID", `#policy=${POLICY_ID}`],
        ["a short Tracking ID", `#id=${TRACKING_ID.slice(0, 9)}&policy=${POLICY_ID}`],
        ["a policy ID that isn't SHA-256", `#id=${TRACKING_ID}&policy=sha512:${"ab".repeat(32)}`],
        ["a short policy ID", `#id=${TRACKING_ID}&policy=sha256:abcd`],
    ])("takes nothing from %s", (_, hash) => {
        expect(parseAuditedFragment(hash)).toBeUndefined()
    })
})

describe("auditedBallotLink", () => {
    it("links to the hub's verifier, with the ballot in the fragment", () => {
        const link = {trackingId: CHARS, policyId: POLICY_ID}
        const url = auditedBallotLink("https://hub.example/verify", IDS, link)
        expect(url).toBe(
            `https://hub.example/verify/tenant/${TENANT_ID}/event/${EVENT_ID}/election/${ELECTION_ID}/audited#id=${TRACKING_ID}&policy=${POLICY_ID}`
        )
        expect(auditedBallotLink("https://hub.example/verify/", IDS, link)).toBe(url)
        expect(parseAuditedFragment(new URL(url).hash)).toEqual({
            trackingId: TRACKING_ID,
            policyId: POLICY_ID,
        })
    })
})

describe("loadAuditedPolicy", () => {
    it("takes the event's policy from its public config when it has the link's ID", async () => {
        const s = auditedScenario()
        net.serve(publicConfigUrl(BUCKET, IDS), config(s.policy))
        const loaded = await loadAuditedPolicy(sdk, BUCKET, IDS, s.policyId, {fetch: net.fetch})
        expect(loaded.ok && loaded.view.policy.id).toBe(s.policyId)
        expect(net.calls).toHaveLength(1)
        expect(net.calls[0].init).toMatchObject({credentials: "omit", redirect: "error"})
    })

    it("takes a pinned policy without asking", async () => {
        const s = auditedScenario()
        localStorage.setItem(PINNED_POLICIES_KEY, JSON.stringify([s.policy]))
        const loaded = await loadAuditedPolicy(sdk, BUCKET, IDS, s.policyId, {fetch: net.fetch})
        expect(loaded.ok).toBe(true)
        expect(net.calls).toEqual([])
    })

    it("refuses a policy with another ID than the link's", async () => {
        const s = auditedScenario()
        net.serve(publicConfigUrl(BUCKET, IDS), config(s.policy))
        expect(await loadAuditedPolicy(sdk, BUCKET, IDS, POLICY_ID, {fetch: net.fetch})).toEqual({
            ok: false,
            problem: EPolicyProblem.UNAVAILABLE,
        })
    })

    it("refuses another event's config", async () => {
        const s = auditedScenario()
        net.serve(
            publicConfigUrl(BUCKET, IDS),
            config(s.policy, {...IDS, eventId: "another-event"})
        )
        expect(await loadAuditedPolicy(sdk, BUCKET, IDS, s.policyId, {fetch: net.fetch})).toEqual({
            ok: false,
            problem: EPolicyProblem.UNAVAILABLE,
        })
    })

    it("says so when the config can't be had", async () => {
        expect(await loadAuditedPolicy(sdk, BUCKET, IDS, POLICY_ID, {fetch: net.fetch})).toEqual({
            ok: false,
            problem: EPolicyProblem.UNAVAILABLE,
        })
    })

    it("refuses a policy that doesn't cover the link's election", async () => {
        const policy = policyText({electionIds: [OTHER_ELECTION_ID]})
        const s = {policy, policyId: policyOf(policy).id}
        net.serve(publicConfigUrl(BUCKET, IDS), config(s.policy))
        expect(await loadAuditedPolicy(sdk, BUCKET, IDS, s.policyId, {fetch: net.fetch})).toEqual({
            ok: false,
            problem: EPolicyProblem.OTHER_ELECTION,
        })
    })
})

describe("hubIdentity", () => {
    const policy = () => policyOf(policyText())

    it("names the hub the settings name, with its operator from the policy", () => {
        expect(hubIdentity(policy(), "h1", "https://any.example/")).toEqual({
            name: "h1",
            operator: "hubco1",
        })
        // A name the policy doesn't have is still the settings' name.
        expect(hubIdentity(policy(), "hub-b", "https://any.example/")).toEqual({name: "hub-b"})
        expect(hubIdentity(undefined, "hub-b", "https://any.example/")).toEqual({name: "hub-b"})
    })

    it("finds the hub whose ballot verifier this page is under", () => {
        const hubs = new Map([
            ["h1", {name: "h1", operator: "hubco1", ballotVerifier: "https://hub1.example/v"}],
            ["h2", {name: "h2", operator: "hubco2", ballotVerifier: "https://hub2.example/v/"}],
        ])
        const withVerifiers = {hubs} as unknown as Policy
        expect(
            hubIdentity(withVerifiers, undefined, "https://hub2.example/v/tenant/t#id=x")
        ).toEqual({name: "h2", operator: "hubco2"})
        // Not a page under it: another path that only starts the same.
        expect(hubIdentity(withVerifiers, undefined, "https://hub1.example/very/")).toEqual({})
    })

    it("names no hub when neither says", () => {
        expect(hubIdentity(policy(), undefined, "https://hub1.example/")).toEqual({})
        expect(hubIdentity(policy(), "  ", "https://hub1.example/")).toEqual({})
    })
})

describe("findBallotEntries", () => {
    const servers = () => bucketServers(policyOf(auditedScenario().policy))

    it("asks the log's own server first, then the monitors", () => {
        expect(servers()).toEqual([
            {server: LOG_SERVER, log: "e/ballots"},
            {server: MONITOR_URL, log: "logs.acme.example/e/ballots"},
            {server: "https://monitor2.example/", log: "logs.acme.example/e/ballots"},
        ])
    })

    it("keeps the ballot whose whole Tracking ID is the link's", async () => {
        const s = auditedScenario()
        net.serve(bucketUrlOf(), bucketAnswer(s.ballots))
        const found = await findBallotEntries(sdk, policyOf(s.policy), TRACKING_ID, IDS, {
            fetch: net.fetch,
        })
        expect(found.kind).toBe(EEntriesKind.FOUND)
        if (found.kind !== EEntriesKind.FOUND) return
        expect(found.match.ballotId).toBe(BALLOT_ID)
        expect(found.match.entries.map((e) => e.stage)).toEqual(["received", "audited"])
        // Only the bucket is asked for: 3 characters of the Tracking ID.
        expect(net.requests()).toEqual([`GET ${bucketUrlOf()}`])
    })

    it("finds nothing when the bucket doesn't list it, or lists it for another election", async () => {
        const s = auditedScenario()
        const policy = policyOf(s.policy)
        const others = new TestLog(KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
        others.add(ballotEntry(OTHER_BALLOT_ID))
        net.serve(bucketUrlOf(), bucketAnswer(others))
        expect(
            (await findBallotEntries(sdk, policy, TRACKING_ID, IDS, {fetch: net.fetch})).kind
        ).toBe(EEntriesKind.NONE)
        net.serve(bucketUrlOf(), bucketAnswer(s.ballots))
        const elsewhere = {...IDS, electionId: OTHER_ELECTION_ID}
        expect(
            (await findBallotEntries(sdk, policy, TRACKING_ID, elsewhere, {fetch: net.fetch})).kind
        ).toBe(EEntriesKind.NONE)
    })

    it("asks a monitor when the log's server doesn't list buckets", async () => {
        const s = auditedScenario()
        net.serve(
            bucketUrlOf(MONITOR_URL.replace(/\/$/, ""), "logs.acme.example/e/ballots"),
            bucketAnswer(s.ballots)
        )
        const found = await findBallotEntries(sdk, policyOf(s.policy), TRACKING_ID, IDS, {
            fetch: net.fetch,
        })
        expect(found.kind).toBe(EEntriesKind.FOUND)
        expect(net.calls).toHaveLength(2)
    })

    it("asks the next server when one stopped at its limit", async () => {
        const s = auditedScenario()
        net.serve(bucketUrlOf(), JSON.stringify({entries: [], truncated: true}))
        net.serve(
            bucketUrlOf(MONITOR_URL.replace(/\/$/, ""), "logs.acme.example/e/ballots"),
            bucketAnswer(s.ballots)
        )
        const found = await findBallotEntries(sdk, policyOf(s.policy), TRACKING_ID, IDS, {
            fetch: net.fetch,
        })
        expect(found.kind).toBe(EEntriesKind.FOUND)
    })

    it("tells servers that don't list buckets from servers that don't answer", async () => {
        const policy = policyOf(auditedScenario().policy)
        expect(
            (await findBallotEntries(sdk, policy, TRACKING_ID, IDS, {fetch: net.fetch})).kind
        ).toBe(EEntriesKind.UNSUPPORTED)
        net.serve(bucketUrlOf(), "busy", 503)
        expect(
            (await findBallotEntries(sdk, policy, TRACKING_ID, IDS, {fetch: net.fetch})).kind
        ).toBe(EEntriesKind.UNAVAILABLE)
    })
})

describe("ballotStage", () => {
    const match = (...stages: string[]): BallotMatch =>
        ({
            ballotId: BALLOT_ID,
            trackingId: TRACKING_ID,
            shared: 13,
            exact: true,
            event: EVENT_ID,
            election: ELECTION_ID,
            entries: stages.map((stage, index) => ({index, stage, acceptedAt: AUDITED_AT + index})),
        }) as BallotMatch

    it("takes the last audit of a ballot that was never cast", () => {
        expect(ballotStage(match("received", "audited", "audited"))).toEqual({
            kind: EStageKind.AUDITED,
            entry: {index: 2, stage: "audited", acceptedAt: AUDITED_AT + 2},
        })
    })

    it("never takes a cast ballot as audited", () => {
        expect(ballotStage(match("received", "audited", "cast")).kind).toBe(EStageKind.CAST)
        expect(ballotStage(match("received")).kind).toBe(EStageKind.PENDING)
    })

    it("rebuilds the audited entry the log holds", () => {
        const s = auditedScenario()
        const policy = policyOf(s.policy)
        const view = {policy} as Parameters<typeof auditedReference>[1]
        const ref = auditedReference(
            sdk,
            view,
            match("received", "audited"),
            {index: s.index, stage: "audited", acceptedAt: AUDITED_AT},
            "https://hub.example/v/tenant/t"
        )
        expect(ref.index).toBe(BigInt(s.index))
        expect(Array.from(ref.entry)).toEqual(Array.from(s.entry))
        expect(ref.log).toBe("ballots")
    })
})

describe("locateBallot", () => {
    const answer = (located: unknown) => JSON.stringify({data: {locate_ballot: located}})
    const locate = () => locateBallot(GRAPHQL, IDS, TRACKING_ID, {fetch: net.fetch})

    it("asks for the whole Tracking ID, with no credentials", async () => {
        net.serve(
            GRAPHQL,
            answer({status: "found", ballot_id: TRACKING_ID, content: "{}"}),
            200,
            "POST"
        )
        expect(await locate()).toEqual({kind: ELocateKind.FOUND, content: "{}"})
        const [call] = net.calls
        expect(call.init).toMatchObject({method: "POST", credentials: "omit"})
        expect(JSON.parse(String(call.init?.body)).variables).toEqual({
            electionEventId: EVENT_ID,
            electionId: ELECTION_ID,
            ballotId: CHARS,
        })
    })

    it("takes no other ballot's record", async () => {
        net.serve(
            GRAPHQL,
            answer({status: "found", ballot_id: OTHER_TRACKING_ID, content: "{}"}),
            200,
            "POST"
        )
        expect((await locate()).kind).toBe(ELocateKind.NOT_FOUND)
        net.serve(GRAPHQL, answer({status: "not-found"}), 200, "POST")
        expect((await locate()).kind).toBe(ELocateKind.NOT_FOUND)
    })

    it("says so when the locator doesn't answer, or refuses", async () => {
        net.serve(GRAPHQL, "", 500, "POST")
        expect((await locate()).kind).toBe(ELocateKind.UNAVAILABLE)
        net.serve(GRAPHQL, JSON.stringify({errors: [{message: "no"}]}), 200, "POST")
        expect((await locate()).kind).toBe(ELocateKind.UNAVAILABLE)
    })
})
