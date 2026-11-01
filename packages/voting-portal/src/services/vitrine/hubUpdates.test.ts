// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import type {Policy} from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {policyText} from "../../fixtures/vitrine"
import {
    auditedBallotLink,
    auditHubs,
    ballotVerifierOf,
    EHubUpdate,
    offeredHubs,
    readHubUpdates,
    shuffled,
    watchLink,
    watchPayload,
} from "./hubUpdates"
import {ETransparencyMode} from "./types"

beforeAll(async () => {
    await verify.init()
})

const SITE_H1 = "https://hub1.explorer.example/"
const SITE_H2 = "https://hub2.explorer.example/#old-fragment"

const parsed = (extra: readonly string[]): Policy =>
    verify.parsePolicy(policyText(ETransparencyMode.PUBLIC, extra))

describe("readHubUpdates", () => {
    it("is on for both when the event says nothing", () => {
        const on = {confirmation: EHubUpdate.ON, castEmail: EHubUpdate.ON}
        expect(readHubUpdates(undefined)).toEqual(on)
        expect(readHubUpdates(null)).toEqual(on)
        expect(readHubUpdates({})).toEqual(on)
        expect(readHubUpdates({hub_updates: null})).toEqual(on)
        expect(readHubUpdates({hub_updates: {}})).toEqual(on)
    })

    it("reads each switch as the admin portal stores it", () => {
        expect(readHubUpdates({hub_updates: {confirmation: "off", cast_email: "on"}})).toEqual({
            confirmation: EHubUpdate.OFF,
            castEmail: EHubUpdate.ON,
        })
        expect(readHubUpdates({hub_updates: {confirmation: "on", cast_email: "off"}})).toEqual({
            confirmation: EHubUpdate.ON,
            castEmail: EHubUpdate.OFF,
        })
    })

    it("keeps a switch that is not set on, and one with a value it does not know", () => {
        expect(readHubUpdates({hub_updates: {confirmation: "off"}}).castEmail).toBe(EHubUpdate.ON)
        expect(readHubUpdates({hub_updates: {cast_email: "off"}}).confirmation).toBe(EHubUpdate.ON)
        expect(readHubUpdates({hub_updates: {confirmation: false}}).confirmation).toBe(
            EHubUpdate.ON
        )
        expect(readHubUpdates({hub_updates: {confirmation: "OFF"}}).confirmation).toBe(
            EHubUpdate.ON
        )
    })

    it("reads a presentation that arrives as its JSON text", () => {
        expect(
            readHubUpdates(JSON.stringify({hub_updates: {confirmation: "off"}})).confirmation
        ).toBe(EHubUpdate.OFF)
        expect(readHubUpdates("not json").confirmation).toBe(EHubUpdate.ON)
    })
})

describe("offeredHubs", () => {
    it("lists the hubs that the policy gives a site, and who runs them", () => {
        const policy = parsed([`hub-explorer h1 ${SITE_H1}`, `hub-explorer h2 ${SITE_H2}`])
        const hubs = offeredHubs(policy)
        expect(hubs).toHaveLength(2)
        expect(hubs).toEqual(
            expect.arrayContaining([
                {name: "h1", party: "hubco1", site: SITE_H1},
                {name: "h2", party: "hubco2", site: SITE_H2},
            ])
        )
    })

    it("leaves out a hub without a site, and every monitor and the operator's explorers", () => {
        const policy = parsed([
            `hub-explorer h2 ${SITE_H2}`,
            "monitor-explorer observers https://monitor.explorer.example/",
            "explorer https://operator.explorer.example/",
        ])
        expect(offeredHubs(policy)).toEqual([{name: "h2", party: "hubco2", site: SITE_H2}])
    })

    it("offers a hub run by the election authority like any other", () => {
        const policy = parsed([`hub-explorer h1 ${SITE_H1}`])
        policy.hubs.get("h1")!.operator = policy.operator
        expect(offeredHubs(policy)).toEqual([{name: "h1", party: "sequent", site: SITE_H1}])
    })

    it("is empty without a policy, or when no hub has a site", () => {
        expect(offeredHubs(undefined)).toEqual([])
        expect(offeredHubs(parsed([]))).toEqual([])
    })
})

describe("shuffled", () => {
    const names = ["a", "b", "c", "d", "e"]

    it("returns the same items without changing the list it was given", () => {
        const before = [...names]
        const result = shuffled(names)
        expect(names).toEqual(before)
        expect([...result].sort()).toEqual(before)
    })

    it("follows the random numbers it is given, so that any order can come up", () => {
        const sequence = (numbers: number[]) => {
            let at = 0
            return () => numbers[at++ % numbers.length]
        }
        expect(shuffled(names, sequence([0.999]))).toEqual(names)
        expect(shuffled(names, sequence([0]))).toEqual(["b", "c", "d", "e", "a"])
        expect(shuffled(names, () => 0.5)).not.toEqual(names)
    })

    it("gives every item every place with the same chance", () => {
        const counts = new Map<string, number>()
        const rounds = 3000
        for (let n = 0; n < rounds; n++) {
            const first = shuffled(["a", "b", "c"])[0]
            counts.set(first, (counts.get(first) ?? 0) + 1)
        }
        for (const first of ["a", "b", "c"]) {
            // 1000 expected; six standard deviations of 25 allow for any run.
            expect(Math.abs((counts.get(first) ?? 0) - rounds / 3)).toBeLessThan(150)
        }
    })
})

describe("watchLink", () => {
    const POLICY_ID = `sha256:${"ab".repeat(32)}`

    it("carries the policy ID and the receipt in the fragment, which the hub's server never sees", () => {
        const link = watchLink(SITE_H1, POLICY_ID, "vitrine/receipt/v1\nline two\n")
        const [base, fragment] = link.split("#")
        expect(base).toBe(SITE_H1)
        const params = new URLSearchParams(fragment)
        expect(params.get("policy")).toBe(POLICY_ID)
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(params.get("watch") ?? ""))).toBe(
            "vitrine/receipt/v1\nline two\n"
        )
        expect(Array.from(params.keys())).toEqual(["policy", "watch"])
    })

    it("drops the fragment of the site's own address", () => {
        const link = watchLink(SITE_H2, POLICY_ID, "x")
        expect(link.startsWith("https://hub2.explorer.example/#policy=")).toBe(true)
        expect(link).not.toContain("old-fragment")
        expect(link.split("#")).toHaveLength(2)
    })

    it("leaves the receipt out until there is one", () => {
        const link = watchLink(SITE_H1, POLICY_ID, undefined)
        expect(new URLSearchParams(link.split("#")[1]).has("watch")).toBe(false)
        expect(new URLSearchParams(link.split("#")[1]).get("policy")).toBe(POLICY_ID)
        expect(watchLink(SITE_H1, POLICY_ID, "")).toBe(link)
    })

    it("keeps to the characters of a base64url fragment", () => {
        const link = watchLink(SITE_H1, POLICY_ID, "é ñ ü — ✓ ??? >>> ~~~")
        expect(link.split("watch=")[1]).toMatch(/^[A-Za-z0-9_-]+$/)
    })
})

describe("watchPayload", () => {
    it("is base64url of the UTF-8 text, with no padding", () => {
        for (const text of ["a", "ab", "abc", "é ñ ü — ✓", "\u{1F5F3}", "??>>~~"]) {
            const payload = watchPayload(text)
            expect(payload).toMatch(/^[A-Za-z0-9_-]*$/)
            expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(payload))).toBe(text)
        }
    })

    it("handles a receipt larger than a function call can spread", () => {
        const text = "line of a receipt chain\n".repeat(20000)
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(watchPayload(text)))).toBe(text)
    })
})

// The vendored parser has no hub-ballot-verifier line yet, so the tests give a
// parsed hub the field the way a newer parser would.
const withVerifier = (policy: Policy, hub: string, url: string): Policy => {
    Object.assign(policy.hubs.get(hub)!, {ballotVerifier: url})
    return policy
}

describe("ballotVerifierOf", () => {
    it("reads the hub's ballot verifier when the policy names one", () => {
        const policy = withVerifier(parsed([]), "h1", "https://hub1.verifier.example/")
        expect(ballotVerifierOf(policy.hubs.get("h1")!)).toBe("https://hub1.verifier.example/")
    })

    it("gives nothing when the hub names no ballot verifier", () => {
        const policy = parsed([])
        expect(ballotVerifierOf(policy.hubs.get("h1")!)).toBeUndefined()
        expect(ballotVerifierOf(withVerifier(policy, "h1", "").hubs.get("h1")!)).toBeUndefined()
    })
})

describe("auditHubs", () => {
    it("lists only the hubs with a ballot verifier, and who runs them", () => {
        const policy = withVerifier(
            parsed([`hub-explorer h1 ${SITE_H1}`, `hub-explorer h2 ${SITE_H2}`]),
            "h2",
            "https://hub2.verifier.example/"
        )
        expect(auditHubs(policy)).toEqual([
            {name: "h2", party: "hubco2", site: "https://hub2.verifier.example/"},
        ])
    })

    it("lists a hub with a ballot verifier but no explorer", () => {
        const policy = withVerifier(parsed([]), "h1", "https://hub1.verifier.example/")
        expect(auditHubs(policy)).toEqual([
            {name: "h1", party: "hubco1", site: "https://hub1.verifier.example/"},
        ])
    })

    it("lists nothing without a policy, or when no hub checks ballots", () => {
        expect(auditHubs(undefined)).toEqual([])
        expect(auditHubs(parsed([`hub-explorer h1 ${SITE_H1}`]))).toEqual([])
    })
})

describe("auditedBallotLink", () => {
    const path = {tenantId: "t-1", eventId: "e-1", electionId: "el-1"}
    const id = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
    const route = "tenant/t-1/event/e-1/election/el-1/audited"

    it("puts the full Tracking ID and the policy in the fragment, after the audited route", () => {
        expect(auditedBallotLink("https://hub.example/", path, id, "sha256:abcd")).toBe(
            `https://hub.example/${route}#id=${id}&policy=sha256:abcd`
        )
    })

    it("adds the slash a base without one is missing", () => {
        expect(auditedBallotLink("https://hub.example/verify", path, id, "sha256:abcd")).toBe(
            `https://hub.example/verify/${route}#id=${id}&policy=sha256:abcd`
        )
    })

    it("replaces a fragment the base already had", () => {
        expect(auditedBallotLink("https://hub.example/#old", path, id, "sha256:abcd")).toBe(
            `https://hub.example/${route}#id=${id}&policy=sha256:abcd`
        )
    })

    it("leaves the policy out when there is none to pin", () => {
        expect(auditedBallotLink("https://hub.example/", path, id)).toBe(
            `https://hub.example/${route}#id=${id}`
        )
    })

    it("encodes ids that are not safe in a path", () => {
        expect(
            auditedBallotLink("https://hub.example/", {...path, eventId: "a/b"}, id, "sha256:ab")
        ).toBe(
            `https://hub.example/tenant/t-1/event/a%2Fb/election/el-1/audited#id=${id}&policy=sha256:ab`
        )
    })
})
