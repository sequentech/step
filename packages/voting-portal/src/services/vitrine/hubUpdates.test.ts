// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import type {Policy} from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {policyText} from "../../fixtures/vitrine"
import {
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
