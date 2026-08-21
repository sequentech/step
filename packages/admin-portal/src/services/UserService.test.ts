// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {UserProfileAttribute} from "@/gql/graphql"
import {
    getInputOptionLabels,
    getSelectOptionDescription,
    getSelectOptionLabel,
    resolveOptionLabel,
} from "./UserService"

const attribute = (overrides: Partial<UserProfileAttribute>): UserProfileAttribute => ({
    name: "sex",
    ...overrides,
})

// Stands in for i18next: a missing key translates to itself.
const translate =
    (messages: Record<string, string> = {}) =>
    (key: string): string =>
        messages[key] ?? key

describe("getInputOptionLabels", () => {
    it("reads a map written straight into the realm configuration", () => {
        expect(
            getInputOptionLabels(attribute({annotations: {inputOptionLabels: {M: "Male"}}}))
        ).toEqual({M: "Male"})
    })

    it("decodes the JSON string Keycloak's admin console stores", () => {
        expect(
            getInputOptionLabels(
                attribute({annotations: {inputOptionLabels: '{"M": "Male", "F": "Female"}'}})
            )
        ).toEqual({M: "Male", F: "Female"})
    })

    it("does not read characters out of a map it could not decode", () => {
        // Indexing the raw string by "0" would yield "{" rather than a description.
        expect(
            getInputOptionLabels(attribute({annotations: {inputOptionLabels: '{"0": "Zero"'}}))
        ).toBeUndefined()
        expect(
            getInputOptionLabels(attribute({annotations: {inputOptionLabels: "Male"}}))
        ).toBeUndefined()
    })

    it("keeps an option whose description is unusable, describing it with itself", () => {
        // Checkbox attributes take their options from these keys, so dropping the
        // entry would drop a selectable value from the form.
        expect(
            getInputOptionLabels(attribute({annotations: {inputOptionLabels: {A: "Alpha", B: 3}}}))
        ).toEqual({A: "Alpha", B: "B"})
    })

    it("ignores a shape that cannot describe options", () => {
        expect(
            getInputOptionLabels(attribute({annotations: {inputOptionLabels: ["Male"]}}))
        ).toBeUndefined()
        expect(getInputOptionLabels(attribute({annotations: {}}))).toBeUndefined()
        expect(getInputOptionLabels(attribute({}))).toBeUndefined()
    })
})

describe("resolveOptionLabel", () => {
    it("localizes a description written as a placeholder", () => {
        expect(resolveOptionLabel("${sex_male}", translate({sex_male: "Hombre"}))).toBe("Hombre")
    })

    it("reads a padded placeholder rather than leaking its braces", () => {
        expect(resolveOptionLabel("  ${sex_male}  ", translate())).toBe("Sex Male")
    })

    it("leaves a description that merely contains a dollar sign alone", () => {
        expect(resolveOptionLabel("Paid $0. Non-member", translate())).toBe("Paid $0. Non-member")
    })
})

describe("getSelectOptionDescription", () => {
    it("reads the description configured for the option", () => {
        expect(getSelectOptionDescription({M: "Male"}, "M", translate())).toBe("Male")
    })

    it("localizes a description written as a placeholder", () => {
        expect(
            getSelectOptionDescription({M: "${sex_male}"}, "M", translate({sex_male: "Hombre"}))
        ).toBe("Hombre")
    })

    it("falls back to a readable name when the placeholder has no override", () => {
        expect(getSelectOptionDescription({M: "${sex_male}"}, "M", translate())).toBe("Sex Male")
    })

    it("leaves a description that merely contains a dollar sign alone", () => {
        expect(getSelectOptionDescription({A: "Paid $0. Non-member"}, "A", translate())).toBe(
            "Paid $0. Non-member"
        )
    })

    it("falls back to a localization override of the option itself", () => {
        expect(getSelectOptionDescription(undefined, "M", translate({M: "Male"}))).toBe("Male")
    })

    it("returns undefined when the option is left undescribed", () => {
        expect(getSelectOptionDescription({F: "Female"}, "M", translate())).toBeUndefined()
        expect(getSelectOptionDescription(undefined, "M", translate())).toBeUndefined()
        // A description that is just the option carries nothing the option does not.
        expect(getSelectOptionDescription({M: "M"}, "M", translate())).toBeUndefined()
    })
})

describe("getSelectOptionLabel", () => {
    it("shows the stored option next to its description", () => {
        expect(getSelectOptionLabel({M: "Male"}, "M", translate())).toBe("M - Male")
    })

    it("keeps the stored option visible when the option itself is overridden", () => {
        expect(getSelectOptionLabel(undefined, "M", translate({M: "Male"}))).toBe("M - Male")
    })

    it("shows the option alone when nothing describes it", () => {
        expect(getSelectOptionLabel(undefined, "M", translate())).toBe("M")
    })

    it("does not repeat a description that only restates the option", () => {
        expect(getSelectOptionLabel({Male: "Male"}, "Male", translate())).toBe("Male")
    })
})
