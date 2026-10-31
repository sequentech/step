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

it("falls back when a legacy translation is inherited or is not text", () => {
    // A property on the prototype is not a translation supplied by this record.
    const dictionary: Record<string, string> = {en: "Council"}
    Object.setPrototypeOf(dictionary, {fr: "Inherited text"})
    expect(translate({name: "Council", name_i18n: dictionary}, "name", "fr")).toBe("Council")
    expect(translate({name: "Council", name_i18n: {fr: 42}}, "name", "fr")).toBe("Council")
    expect(translate({name: 42}, "name", "fr")).toBeUndefined()
})

it("handles absent data and an empty language without losing a legacy field", () => {
    expect(translateFromPresentation(null, "name", "en")).toBeUndefined()
    expect(translateFromPresentation(undefined, "name", "en")).toBeUndefined()
    expect(translateFromPresentation({name: "Council", presentation: null}, "name", "")).toBe(
        "Council"
    )
    expect(translateFromPresentation({name: ""}, "name", "en")).toBeUndefined()
})

it("requires own translation containers and fallback fields at every level", () => {
    const dictionary = {fr: "Conseil"}
    expect(translate({name: "Council", name_i18n: dictionary}, "name", "fr")).toBe("Conseil")
    const legacy = Object.assign(Object.create({name_i18n: dictionary}), {name: "Council"})
    expect(translate(legacy, "name", "fr")).toBe("Council")
    expect(translate(Object.create({name: "Inherited"}), "name", "fr")).toBeUndefined()

    const presentation = {i18n: {fr: {name: "Conseil"}}}
    expect(translateFromPresentation({name: "Council", presentation}, "name", "fr")).toBe("Conseil")
    for (const inherited of [
        Object.assign(Object.create({presentation}), {name: "Council"}),
        {name: "Council", presentation: Object.create(presentation)},
    ]) {
        expect(translateFromPresentation(inherited, "name", "fr")).toBe("Council")
    }
    expect(
        translateFromPresentation(Object.create({name: "Inherited"}), "name", "fr")
    ).toBeUndefined()
    expect(isTranslatablePresentation(Object.create(presentation))).toBe(false)
})
