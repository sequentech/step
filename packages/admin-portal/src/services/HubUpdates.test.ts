/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The "Updates from hubs" settings of an election event (design board
// AdminHubUpdates): two switches, on by default, stored in the event's
// presentation, and the hubs the policy offers voters.

import * as verify from "@vitrine/verify"
import {draftPolicyText, HUB_EXPLORERS, policyText} from "@/fixtures/vitrinePolicy"
import {EHubUpdate, hubUpdatesPatch, offeredHubs, readHubUpdates} from "./HubUpdates"

beforeAll(async () => {
    await verify.init()
})

describe("reading the switches", () => {
    it("reads both as on when the event has never set them", () => {
        for (const presentation of [undefined, null, {}, {hub_updates: null}, "nonsense"]) {
            expect(readHubUpdates(presentation)).toEqual({
                confirmation: EHubUpdate.ON,
                castEmail: EHubUpdate.ON,
            })
        }
    })

    it("reads each as the presentation says", () => {
        expect(readHubUpdates({hub_updates: {confirmation: "off", cast_email: "on"}})).toEqual({
            confirmation: EHubUpdate.OFF,
            castEmail: EHubUpdate.ON,
        })
        expect(readHubUpdates({hub_updates: {confirmation: "on", cast_email: "off"}})).toEqual({
            confirmation: EHubUpdate.ON,
            castEmail: EHubUpdate.OFF,
        })
    })

    it("keeps a switch that is not set on, and one that says something else", () => {
        expect(readHubUpdates({hub_updates: {confirmation: "off"}})).toEqual({
            confirmation: EHubUpdate.OFF,
            castEmail: EHubUpdate.ON,
        })
        expect(readHubUpdates({hub_updates: {confirmation: false, cast_email: "maybe"}})).toEqual({
            confirmation: EHubUpdate.ON,
            castEmail: EHubUpdate.ON,
        })
    })

    it("reads a presentation given as its JSON text", () => {
        expect(readHubUpdates(JSON.stringify({hub_updates: {cast_email: "off"}}))).toEqual({
            confirmation: EHubUpdate.ON,
            castEmail: EHubUpdate.OFF,
        })
    })
})

describe("what is stored", () => {
    it("writes both switches under hub_updates, as the Rust presentation spells them", () => {
        expect(hubUpdatesPatch({confirmation: EHubUpdate.OFF, castEmail: EHubUpdate.ON})).toEqual({
            hub_updates: {confirmation: "off", cast_email: "on"},
        })
    })
})

describe("the hubs voters are offered", () => {
    const sealed = (hubExplorers: boolean) => verify.pinPolicy(policyText({hubExplorers}))

    it("are the hubs of the policy that have an explorer, with who runs them, in policy order", () => {
        expect(offeredHubs(sealed(true))).toEqual([
            {name: "h1", party: "hubco1", site: HUB_EXPLORERS.h1},
            {name: "h2", party: "hubco2", site: HUB_EXPLORERS.h2},
        ])
    })

    it("are none when no hub has an explorer, or when there is no policy yet", () => {
        expect(offeredHubs(sealed(false))).toEqual([])
        expect(offeredHubs(undefined)).toEqual([])
    })

    it("come from a draft policy too, so the admin sees them before it is sealed", () => {
        const draft = verify.parsePolicy(draftPolicyText({hubExplorers: true}), {
            allowDraft: true,
            requireSignatures: false,
        })
        expect(offeredHubs(draft).map((hub) => hub.name)).toEqual(["h1", "h2"])
    })
})
