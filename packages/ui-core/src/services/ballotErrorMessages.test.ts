// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import i18next from "i18next"
import {getBallotErrorOptions} from "./ballotErrorMessages"

describe("getBallotErrorOptions", () => {
    it("accepts every shape a message_map can cross the WASM boundary in", () => {
        const expected = {numSelected: 2, max: 4, count: 2}

        expect(
            getBallotErrorOptions("errors.implicit.underVote", {numSelected: "2", max: "4"})
        ).toEqual(expected)
        expect(
            getBallotErrorOptions(
                "errors.implicit.underVote",
                new Map([
                    ["numSelected", "2"],
                    ["max", "4"],
                ])
            )
        ).toEqual(expected)
        expect(
            getBallotErrorOptions("errors.implicit.underVote", [
                ["numSelected", "2"],
                ["max", "4"],
            ])
        ).toEqual(expected)
    })

    it("returns an empty object for a missing or unusable message_map", () => {
        expect(getBallotErrorOptions("errors.implicit.blankVote", undefined)).toEqual({})
        expect(getBallotErrorOptions("errors.implicit.blankVote", null)).toEqual({})
        expect(getBallotErrorOptions("errors.implicit.blankVote", "not a map")).toEqual({})
        expect(getBallotErrorOptions(undefined, {numSelected: "2"})).toEqual({numSelected: 2})
    })

    it("coerces numeric values and leaves the rest as strings", () => {
        const options = getBallotErrorOptions("errors.implicit.maxSelectionsPerType", {
            numSelected: "3",
            max: "1",
            type: "statewide-officers",
        })

        expect(options.numSelected).toBe(3)
        expect(options.max).toBe(1)
        expect(options.type).toBe("statewide-officers")
    })

    it("derives count as the action left to take, not as the state observed", () => {
        // 2 of a required minimum of 5: five minus two still to select.
        expect(
            getBallotErrorOptions("errors.implicit.selectedMin", {numSelected: "2", min: "5"}).count
        ).toBe(3)
        // 3 of an allowed maximum of 4: one more may still be selected.
        expect(
            getBallotErrorOptions("errors.implicit.underVote", {numSelected: "3", max: "4"}).count
        ).toBe(1)
        // 5 where 3 are allowed: two have to go.
        expect(
            getBallotErrorOptions("errors.implicit.selectedMax", {numSelected: "5", max: "3"}).count
        ).toBe(2)
        expect(
            getBallotErrorOptions("errors.implicit.maxSelectionsPerType", {
                numSelected: "3",
                max: "1",
                type: "party-a",
            }).count
        ).toBe(2)
        // Already at the maximum: the sentence is about the maximum itself.
        expect(
            getBallotErrorOptions("errors.implicit.overVoteDisabled", {
                numSelected: "4",
                max: "4",
            }).count
        ).toBe(4)
    })

    it("never emits a negative or non-numeric count", () => {
        expect(
            getBallotErrorOptions("errors.implicit.underVote", {numSelected: "4", max: "4"}).count
        ).toBe(0)
        expect(getBallotErrorOptions("errors.implicit.underVote", {}).count).toBe(0)
        expect(
            getBallotErrorOptions("errors.implicit.selectedMin", {numSelected: "x", min: "y"}).count
        ).toBe(0)
    })

    it("passes an existing count through as a number", () => {
        expect(
            getBallotErrorOptions("errors.configuration.multipleExplicitBlankCandidates", {
                count: "2",
            })
        ).toEqual({count: 2})
    })
})

/**
 * The reason the helper has to coerce at all: i18next only pluralises when
 * `count` is a number, and the plural category comes from `Intl.PluralRules`,
 * not from `count === 1`.
 */
describe("i18next plural selection", () => {
    const t = i18next.createInstance()

    beforeAll(async () => {
        await t.init({
            lng: "en",
            fallbackLng: "en",
            ns: ["translations"],
            defaultNS: "translations",
            keySeparator: ".",
            interpolation: {escapeValue: false},
            resources: {
                en: {
                    translations: {
                        selected_one: "{{count}} candidate selected",
                        selected_other: "{{count}} candidates selected",
                    },
                },
                fr: {
                    translations: {
                        selected_one: "{{count}} candidat sélectionné",
                        selected_other: "{{count}} candidats sélectionnés",
                    },
                },
                tl: {
                    translations: {
                        selected_one: "{{count}} kandidatong napili",
                        selected_other: "{{count}} na kandidatong napili",
                    },
                },
            },
        })
    })

    it("skips plural handling when count is a string", () => {
        expect(t.t("selected", {count: 2})).toBe("2 candidates selected")
        expect(t.t("selected", {count: "2"})).not.toBe("2 candidates selected")
    })

    it("uses the plural rules of the language, which are not always count === 1", async () => {
        // French counts zero as singular.
        await t.changeLanguage("fr")
        expect(t.t("selected", {count: 0})).toBe("0 candidat sélectionné")
        expect(t.t("selected", {count: 1})).toBe("1 candidat sélectionné")
        expect(t.t("selected", {count: 2})).toBe("2 candidats sélectionnés")

        // Filipino counts two as singular.
        await t.changeLanguage("tl")
        expect(t.t("selected", {count: 2})).toBe("2 kandidatong napili")

        await t.changeLanguage("en")
        expect(t.t("selected", {count: 0})).toBe("0 candidates selected")
    })
})
