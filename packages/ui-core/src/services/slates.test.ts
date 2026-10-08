// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EMobileCandidateLists, ISlatesConfig} from "../types/Slates"
import {getCandidateSlate, getSlateName, hasSlateMembers} from "./slates"

const CONFIG: ISlatesConfig = {
    version: 1,
    mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
    slates: [
        {
            id: "forward",
            name: {en: "Forward Together", es: "Adelante Juntos"},
            members: {president: ["f-president"], trustees: ["f-t1", "f-t2"]},
        },
        {
            id: "voices",
            name: {fr: "Voix Indépendantes", es: "Voces Independientes"},
            members: {trustees: ["v-t1"]},
        },
    ],
}

describe("getSlateName", () => {
    const [forward, voices] = CONFIG.slates

    it("uses the voter's language", () => {
        expect(getSlateName(forward, "es")).toBe("Adelante Juntos")
    })

    it("matches a regional language by its primary subtag", () => {
        expect(getSlateName(forward, "es-MX")).toBe("Adelante Juntos")
    })

    it("falls back to the default language", () => {
        expect(getSlateName(forward, "fr", "en")).toBe("Forward Together")
    })

    it("falls back to the first language the slate is named in", () => {
        expect(getSlateName(voices, "en", "nl")).toBe("Voces Independientes")
    })

    it("is empty for a slate without a name", () => {
        expect(getSlateName({name: {}}, "en")).toBe("")
    })
})

describe("getCandidateSlate", () => {
    it("finds the slate of a member", () => {
        expect(getCandidateSlate(CONFIG, "trustees", "v-t1")?.id).toBe("voices")
    })

    it("has no slate for an independent candidate", () => {
        expect(getCandidateSlate(CONFIG, "trustees", "i-t1")).toBeUndefined()
    })

    it("does not match a member under another contest", () => {
        expect(getCandidateSlate(CONFIG, "president", "f-t1")).toBeUndefined()
    })
})

describe("hasSlateMembers", () => {
    it("is true for a contest some slate covers", () => {
        expect(hasSlateMembers(CONFIG, "president")).toBe(true)
    })

    it("is false for a contest no slate covers", () => {
        expect(hasSlateMembers(CONFIG, "referendum")).toBe(false)
    })
})
