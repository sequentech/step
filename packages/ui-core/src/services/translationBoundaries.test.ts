// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, it} from "@jest/globals"
import {translate, translateFromPresentation, isTranslatablePresentation} from "./translate"

it("supports legacy translated fields while retaining the original value as fallback", () => {
    const entity = {name: "Council", name_i18n: {fr: "Conseil", en: ""}}
    expect(translate(entity, "name", "fr")).toBe("Conseil")
    expect(translate(entity, "name", "en")).toBe("")
    expect(translate(entity, "name", "de")).toBe("Council")
    expect(translate({name: "Council"}, "name", "fr")).toBe("Council")
})

it("validates translation containers without accepting arrays or nontext leaf values", () => {
    for (const malformed of [
        null,
        [],
        1,
        {i18n: []},
        {i18n: "bad"},
        {i18n: {en: []}},
        {i18n: {en: {name: 3}}},
    ]) {
        expect(isTranslatablePresentation(malformed)).toBe(false)
    }
    for (const valid of [
        {},
        {i18n: {}},
        {i18n: {en: {name: "Council", note: null, title: undefined}}},
    ]) {
        expect(isTranslatablePresentation(valid)).toBe(true)
    }
})

it("handles absent data and an empty language without losing a legacy field", () => {
    expect(translateFromPresentation(null, "name", "en")).toBeUndefined()
    expect(translateFromPresentation(undefined, "name", "en")).toBeUndefined()
    expect(translateFromPresentation({name: "Council", presentation: null}, "name", "")).toBe(
        "Council"
    )
    expect(translateFromPresentation({name: ""}, "name", "en")).toBeUndefined()
})
