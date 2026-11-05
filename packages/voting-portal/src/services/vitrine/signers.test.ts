// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import type {
    ChainResult,
    HubTierDetail,
    Policy,
    TierName,
    TierResult,
    TierStatus,
} from "@vitrine/verify"
import {
    ballotEntry,
    chainText,
    KEYS,
    LOG_URLS,
    NOW,
    policyText,
    TestLog,
} from "../../fixtures/vitrine"
import {
    ESignerKind,
    ESignerState,
    isOperatorOrPlatform,
    platformOf,
    signerLists,
    signerTimes,
    soleHubOf,
    type ISignerTimes,
} from "./signers"
import {ETransparencyMode} from "./types"

beforeAll(async () => {
    await verify.init()
})

const EXPLORER_OBSERVERS = "https://explorer.observers.example/"
const EXPLORER_H2 = "https://explorer.h2.example/"
const EXPLORER_SEQUENT = "https://explorer.sequent.example/"

/** w1 runs by itself, w3 by hubco1, which also runs h1. Monitor observers
 * (monitorco1) and observers2 (monitorco2) and hub h2 (hubco2) name a site. */
const POLICY_LINES = [
    "witness-party w3 hubco1",
    `monitor-explorer observers ${EXPLORER_OBSERVERS}`,
    `monitor-explorer observers2 ${EXPLORER_SEQUENT}`,
    `hub-explorer h2 ${EXPLORER_H2}`,
    "anchor-hubs 2 1",
]

/** The parsed policy, with some parties changed afterwards: a policy that lets
 * the operator run a witness of a 2 of 3 quorum is not a valid one. */
const parsedPolicy = (
    extra: readonly string[] = POLICY_LINES,
    change: (policy: Policy) => void = (policy) => {
        policy.witnesses.witnesses.get("w2")!.party = "sequent"
    }
): Policy => {
    const policy = verify.parsePolicy(policyText(ETransparencyMode.PUBLIC, extra))
    change(policy)
    return policy
}

const witnessParties =
    (parties: Record<string, string>) =>
    (policy: Policy): void => {
        for (const [name, party] of Object.entries(parties)) {
            policy.witnesses.witnesses.get(name)!.party = party
        }
    }

const tier = (
    name: TierName,
    status: TierStatus,
    hubs?: Record<string, TierStatus>
): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...(hubs
        ? {
              hubs: Object.entries(hubs).map(
                  ([hub, state]): HubTierDetail => ({hub, status: state, reasons: []})
              ),
          }
        : {}),
})

const resultOf = (tiers: TierResult[], browser: ChainResult["policy"]["browser"] = "bootstrap") =>
    ({
        ok: true,
        status: "checking",
        tiers,
        steps: [],
        entry: {log: "ballots", leafHash: "ab", index: 1, promised: false},
        policy: {
            id: "p",
            event: "e",
            witnesses: [],
            hubs: ["h1", "h2"],
            browser,
        },
        now: NOW,
    }) as unknown as ChainResult

const TIMES: ISignerTimes = {
    witnesses: new Map([
        ["w1", NOW - 3],
        ["w2", NOW - 2],
    ]),
    hubs: new Map([["h1", NOW - 1]]),
    size: 22,
}

const VERIFIED_TIERS = [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified", {h1: "verified", h2: "verified"}),
    tier("browser", "verified", {h1: "verified", h2: "verified"}),
]

const fragment = (href: string | undefined): URLSearchParams =>
    new URLSearchParams((href ?? "").split("#")[1] ?? "")

describe("signerTimes", () => {
    const withRecord = (cosigners: readonly {key: (typeof KEYS.witnesses)[0]; time: number}[]) => {
        const policy = parsedPolicy()
        const log = new TestLog("ballots", KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
        const index = log.add(ballotEntry("aa".repeat(32)))
        log.add(ballotEntry("bb".repeat(32)))
        const record = pure.bytes.utf8(log.receipt(index, 2, cosigners))
        const chain = chainText(policy.id, "ballots", {record})
        return {policy, chain}
    }

    it("reads when each witness cosigned the record's checkpoint", () => {
        const {policy, chain} = withRecord([
            {key: KEYS.witnesses[0], time: NOW - 9},
            {key: KEYS.witnesses[2], time: NOW - 7},
        ])
        const times = signerTimes(pure, {policy, chain, now: NOW})
        expect(times?.witnesses).toEqual(
            new Map([
                ["w1", NOW - 9],
                ["w3", NOW - 7],
            ])
        )
        expect(times?.size).toBe(2)
        expect(times?.hubs.size).toBe(0)
    })

    it("leaves out a cosignature from a key the policy does not list", () => {
        const {policy, chain} = withRecord([
            {key: KEYS.witnesses[1], time: NOW - 9},
            {key: KEYS.hubWitness1, time: NOW - 8},
        ])
        const times = signerTimes(pure, {policy, chain, now: NOW})
        expect(Array.from(times?.witnesses.keys() ?? [])).toEqual(["w2"])
    })

    it("reads the receipt issued at the cast as well, until a later record meets the quorum", () => {
        const policy = parsedPolicy()
        const log = new TestLog("ballots", KEYS.ballots, KEYS.ballotsTime, LOG_URLS.ballots)
        const index = log.add(ballotEntry("aa".repeat(32)))
        log.add(ballotEntry("bb".repeat(32)))
        log.add(ballotEntry("cc".repeat(32)))
        const issued = pure.bytes.utf8(
            log.receipt(index, 2, [{key: KEYS.witnesses[0], time: NOW - 9}])
        )
        const issuedOnly = chainText(policy.id, "ballots", {issued})
        expect(signerTimes(pure, {policy, chain: issuedOnly, now: NOW})?.witnesses).toEqual(
            new Map([["w1", NOW - 9]])
        )
        // The later record, at size 3, is the one that meets the quorum of 2.
        const record = pure.bytes.utf8(
            log.receipt(index, 3, [
                {key: KEYS.witnesses[0], time: NOW - 5},
                {key: KEYS.witnesses[2], time: NOW - 4},
            ])
        )
        const both = signerTimes(pure, {
            policy,
            chain: chainText(policy.id, "ballots", {issued, record}),
            now: NOW,
        })
        expect(both?.witnesses).toEqual(
            new Map([
                ["w1", NOW - 5],
                ["w3", NOW - 4],
            ])
        )
        expect(both?.size).toBe(3)
    })

    it("reads nothing from a chain it cannot parse or that has no receipt", () => {
        const {policy} = withRecord([])
        expect(signerTimes(pure, {policy, chain: "not a chain", now: NOW})).toBeUndefined()
        expect(
            signerTimes(pure, {policy, chain: chainText(policy.id, "ballots", {}), now: NOW})
        ).toBeUndefined()
    })

    /** The verifier with a chain that holds these hubs' links, and a hub link check. */
    const withHubs = (
        names: readonly string[],
        verifyHubLink: (link: Uint8Array | string, hub: {name: string}) => unknown
    ) => {
        const {policy, chain} = withRecord([{key: KEYS.witnesses[0], time: NOW - 9}])
        const parsed = pure.parseChain(chain)
        const hubs = new Map(names.map((name) => [name, {anchor: pure.bytes.utf8(`link ${name}`)}]))
        const fake = {
            ...pure,
            parseChain: () => ({...parsed, hubs}),
            hubs: {...pure.hubs, verifyHubLink},
        } as unknown as typeof pure
        return {policy, chain, fake}
    }

    it("reads a hub's time from its link when the hub's quorum signed it, and skips one that does not verify", () => {
        const verifyHubLink = jest.fn((_link: Uint8Array | string, hub: {name: string}) => {
            if (hub.name === "h2") throw new Error("bad link")
            return {hubQuorum: true, hubQuorumTime: NOW - 4}
        })
        // h3 is not a hub of the policy.
        const {policy, chain, fake} = withHubs(["h1", "h2", "h3"], verifyHubLink)
        const times = signerTimes(fake, {policy, chain, now: NOW})
        expect(times?.hubs).toEqual(new Map([["h1", NOW - 4]]))
        expect(verifyHubLink).toHaveBeenCalledTimes(2)
    })

    it("skips a hub link whose hub quorum is not met", () => {
        const {policy, chain, fake} = withHubs(["h1"], () => ({
            hubQuorum: false,
            hubQuorumTime: undefined,
        }))
        expect(signerTimes(fake, {policy, chain, now: NOW})?.hubs.size).toBe(0)
    })
})

describe("who runs a signer", () => {
    it("names no platform when the policy has no platform line", () => {
        expect(platformOf(parsedPolicy())).toBeUndefined()
    })

    it("names the party of the policy's platform line", () => {
        const policy = parsedPolicy([...POLICY_LINES, "platform hosting-co"])
        expect(platformOf(policy)).toBe("hosting-co")
        expect(isOperatorOrPlatform(policy, "hosting-co")).toBe(true)
        expect(isOperatorOrPlatform(parsedPolicy(), "hosting-co")).toBe(false)
    })

    it("takes the operator and the platform as the ones that get no link", () => {
        const policy = parsedPolicy()
        expect(isOperatorOrPlatform(policy, "sequent")).toBe(true)
        expect(isOperatorOrPlatform(policy, "hubco1")).toBe(false)
        expect(isOperatorOrPlatform(policy, "hubco1", "hubco1")).toBe(true)
    })
})

describe("the witnesses list", () => {
    const list = (
        overrides: Partial<Parameters<typeof signerLists>[0]> = {}
    ): ReturnType<typeof signerLists>["witnesses"] =>
        signerLists({
            policy: parsedPolicy(),
            result: resultOf(VERIFIED_TIERS),
            times: TIMES,
            standIn: false,
            ...overrides,
        }).witnesses

    it("lists the policy's witnesses in its order, with who runs each, from the policy", () => {
        const witnesses = list()
        expect(witnesses?.kind).toBe(ESignerKind.WITNESSES)
        expect(witnesses?.rows.map((r) => [r.name, r.party])).toEqual([
            ["w1", "w1"],
            ["w2", "sequent"],
            ["w3", "hubco1"],
        ])
    })

    it("says which hub a witness's party also runs", () => {
        const rows = list()?.rows ?? []
        expect(rows[2]?.alsoRuns).toEqual(["h1"])
        expect(rows[0]?.alsoRuns).toEqual([])
    })

    it("shows the time of those that signed, Not yet for the others", () => {
        const rows = list()?.rows ?? []
        expect(rows.map((r) => [r.state, r.at])).toEqual([
            [ESignerState.DONE, NOW - 3],
            [ESignerState.DONE, NOW - 2],
            [ESignerState.PENDING, undefined],
        ])
    })

    it("shows an unsigned witness as Late when the tier is late, never as a failure", () => {
        const late = list({
            result: resultOf([
                tier("record", "verified"),
                tier("witnesses", "late"),
                tier("hubs", "pending", {h1: "pending", h2: "pending"}),
                tier("browser", "pending"),
            ]),
        })
        expect(late?.rows.map((r) => r.state)).toEqual([
            ESignerState.DONE,
            ESignerState.DONE,
            ESignerState.LATE,
        ])
    })

    it("counts those that signed against the policy's quorum, and says no one party can confirm alone", () => {
        expect(list()?.rule).toEqual({
            key: "vitrine.signers.rule.witnessesAlone",
            values: {signed: 2, total: 3, needed: 2},
        })
    })

    it("does not claim that no one party can confirm alone when one party runs enough witnesses", () => {
        const policy = parsedPolicy(POLICY_LINES, witnessParties({w1: "hubco1", w2: "hubco1"}))
        const witnesses = list({policy})
        expect(witnesses?.rule.key).toBe("vitrine.signers.rule.witnesses")
        expect(witnesses?.rule.values).toEqual({signed: 2, total: 3, needed: 2})
    })

    it("links a witness only to a site the policy names for its party, once it signed", () => {
        const named = list({
            policy: parsedPolicy(POLICY_LINES, (policy) => {
                witnessParties({w1: "monitorco1", w2: "sequent", w3: "hubco2"})(policy)
                // The operator names a site too, through a monitor it runs.
                policy.monitors.get("observers2")!.party = "sequent"
            }),
        })
        const [w1, w2, w3] = named?.rows ?? []
        expect(w1?.site?.startsWith(`${EXPLORER_OBSERVERS}#`)).toBe(true)
        const query = fragment(w1?.site)
        expect(query.get("policy")).toBe(parsedPolicy().id)
        expect(query.get("log")).toBe("ballots")
        expect(query.get("checkpoint")).toBe("22")
        expect(query.get("cosigner")).toBe("w1")
        // w2 is run by the operator, whose site gets no link.
        expect(w2?.state).toBe(ESignerState.DONE)
        expect(w2?.site).toBeUndefined()
        // w3 has not signed yet: no link before it does.
        expect(w3?.state).toBe(ESignerState.PENDING)
        expect(w3?.site).toBeUndefined()
    })

    it("gives no link to a party the policy says runs the platform", () => {
        const policy = parsedPolicy(POLICY_LINES, witnessParties({w1: "monitorco1"}))
        expect(list({policy})?.rows[0]?.site).toContain(EXPLORER_OBSERVERS)
        expect(list({policy, platform: "monitorco1"})?.rows[0]?.site).toBeUndefined()
    })

    it("is not shown without who signed, a quorum check, or a policy", () => {
        expect(list({times: undefined})).toBeUndefined()
        expect(list({result: undefined})).toBeUndefined()
        expect(list({policy: undefined})).toBeUndefined()
    })
})

describe("the hubs list", () => {
    const hubs = (overrides: Partial<Parameters<typeof signerLists>[0]> = {}) =>
        signerLists({
            policy: parsedPolicy(),
            result: resultOf(VERIFIED_TIERS),
            times: TIMES,
            standIn: false,
            ...overrides,
        }).hubs

    it("lists the policy's hubs with who runs each, and when they anchored", () => {
        const list = hubs()
        expect(list?.kind).toBe(ESignerKind.HUBS)
        expect(list?.rows.map((r) => [r.name, r.party, r.state, r.at])).toEqual([
            ["h1", "hubco1", ESignerState.DONE, NOW - 1],
            ["h2", "hubco2", ESignerState.DONE, undefined],
        ])
    })

    it("shows a hub that has not anchored as pending, and as late once its deadline passed", () => {
        const pending = hubs({
            result: resultOf([
                tier("record", "verified"),
                tier("witnesses", "verified"),
                tier("hubs", "pending", {h1: "verified", h2: "pending"}),
                tier("browser", "pending"),
            ]),
        })
        expect(pending?.rows.map((r) => r.state)).toEqual([ESignerState.DONE, ESignerState.PENDING])
        const late = hubs({
            result: resultOf([
                tier("record", "verified"),
                tier("witnesses", "verified"),
                tier("hubs", "late", {h1: "verified", h2: "late"}),
                tier("browser", "pending"),
            ]),
        })
        expect(late?.rows.map((r) => r.state)).toEqual([ESignerState.DONE, ESignerState.LATE])
    })

    it("links a hub that names a site and is not run by the operator, once it anchored", () => {
        const [h1, h2] = hubs()?.rows ?? []
        expect(h1?.site).toBeUndefined()
        const query = fragment(h2?.site)
        expect(h2?.site?.startsWith(`${EXPLORER_H2}#`)).toBe(true)
        expect(query.get("anchor")).toBe("h2")
        expect(query.get("checkpoint")).toBe("22")
        expect(query.get("cosigner")).toBeNull()
        const pending = hubs({
            result: resultOf([
                tier("record", "verified"),
                tier("witnesses", "verified"),
                tier("hubs", "pending", {h1: "verified", h2: "pending"}),
                tier("browser", "pending"),
            ]),
        })
        expect(pending?.rows[1]?.site).toBeUndefined()
    })

    it("lists where browsers check as a stand-in hub only when asked and the policy has one", () => {
        const withStandIn = hubs({standIn: true})
        expect(withStandIn?.rows.map((r) => r.standIn ?? false)).toEqual([false, false, true])
        expect(withStandIn?.rows[2]?.state).toBe(ESignerState.DONE)
        expect(hubs({standIn: false})?.rows).toHaveLength(2)
        const noStandIn = hubs({standIn: true, result: resultOf(VERIFIED_TIERS, "waict")})
        expect(noStandIn?.rows).toHaveLength(2)
    })

    it("says what the policy needs, and that at least one hub is run by someone else", () => {
        expect(hubs()?.rule).toEqual({
            key: "vitrine.signers.rule.hubs",
            values: {count: 2, independent: 1},
        })
    })

    it("says the platform too, once the policy names one", () => {
        expect(hubs({platform: "sequent"})?.rule.key).toBe("vitrine.signers.rule.hubsPlatform")
    })

    it("says only the count when the policy asks for no hub run by someone else", () => {
        const policy = parsedPolicy(["anchor-hubs 2 0"])
        expect(hubs({policy})?.rule).toEqual({
            key: "vitrine.signers.rule.hubsAll",
            values: {count: 2},
        })
    })

    describe("with one hub", () => {
        const base = parsedPolicy()
        const one = {...base, hubs: new Map([["h1", base.hubs.get("h1")!]])} as Policy
        const alone = (overrides: Partial<Parameters<typeof signerLists>[0]> = {}) =>
            hubs({
                policy: one,
                result: resultOf([
                    tier("record", "verified"),
                    tier("witnesses", "verified"),
                    tier("hubs", "verified", {h1: "verified"}),
                    tier("browser", "verified"),
                ]),
                ...overrides,
            })

        it("finds the only hub and who runs it", () => {
            expect(soleHubOf(one)).toEqual({name: "h1", party: "hubco1", platform: false})
            expect(soleHubOf(one, "hubco1")).toEqual({name: "h1", party: "hubco1", platform: true})
            expect(soleHubOf(base)).toBeUndefined()
        })

        it("says that it is anchored in 1 hub, and who runs it", () => {
            expect(alone()?.rule).toEqual({
                key: "vitrine.signers.rule.oneHub",
                values: {party: "hubco1"},
            })
        })

        it("says it is run by the platform when the policy names the platform as its operator", () => {
            expect(alone({platform: "hubco1"})?.rule.key).toBe(
                "vitrine.signers.rule.oneHubPlatform"
            )
        })
    })

    it("is not shown when the policy has no hub", () => {
        const noHubs = resultOf([
            tier("record", "verified"),
            tier("witnesses", "verified"),
            tier("hubs", "unused"),
            tier("browser", "unused"),
        ])
        expect(hubs({result: noHubs})).toBeUndefined()
    })
})
