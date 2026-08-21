// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {UserProfileAttribute} from "@/gql/graphql"
import {
    getAttributeLengthBounds,
    getAttributeLengthViolation,
    getAttributeViolation,
    getInputOptionLabels,
    getStatedLengthBounds,
    isHiddenAttribute,
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

describe("getAttributeLengthBounds", () => {
    it("reads both bounds from the length validator", () => {
        expect(
            getAttributeLengthBounds(attribute({validations: {length: {min: 1, max: 2}}}))
        ).toEqual({min: 1, max: 2, trim: true})
    })

    it("reads bounds Keycloak stored as strings", () => {
        expect(
            getAttributeLengthBounds(attribute({validations: {length: {min: "1", max: "2"}}}))
        ).toEqual({min: 1, max: 2, trim: true})
    })

    it("reads a single bound", () => {
        expect(getAttributeLengthBounds(attribute({validations: {length: {max: 40}}}))).toEqual({
            min: undefined,
            max: 40,
            trim: true,
        })
    })

    it("follows the attribute when it opts out of trimming", () => {
        expect(
            getAttributeLengthBounds(
                attribute({validations: {length: {"max": 2, "trim-disabled": "true"}}})
            )?.trim
        ).toBe(false)
        expect(
            getAttributeLengthBounds(
                attribute({validations: {length: {"max": 2, "trim-disabled": true}}})
            )?.trim
        ).toBe(false)
    })

    it("returns undefined when nothing is bounded", () => {
        expect(getAttributeLengthBounds(attribute({validations: {length: {}}}))).toBeUndefined()
        expect(
            getAttributeLengthBounds(attribute({validations: {options: {options: ["a"]}}}))
        ).toBeUndefined()
        expect(getAttributeLengthBounds(attribute({}))).toBeUndefined()
    })

    // The annotation caps typing on the voter-facing forms but is not enforced
    // on submit, so stating it would claim a rule the server does not apply.
    it("ignores the inputTypeMaxlength annotation", () => {
        expect(
            getAttributeLengthBounds(attribute({annotations: {inputTypeMaxlength: 5}}))
        ).toBeUndefined()
    })
})

describe("getAttributeLengthViolation", () => {
    const bounds = getAttributeLengthBounds(attribute({validations: {length: {min: 2, max: 4}}}))

    it("names the bound the value breaks", () => {
        expect(getAttributeLengthViolation(bounds, "a")).toBe("tooShort")
        expect(getAttributeLengthViolation(bounds, "abcde")).toBe("tooLong")
    })

    it("accepts a value within the bounds, including at them", () => {
        expect(getAttributeLengthViolation(bounds, "ab")).toBeUndefined()
        expect(getAttributeLengthViolation(bounds, "abcd")).toBeUndefined()
    })

    it("leaves an absent value to the attribute being required", () => {
        expect(getAttributeLengthViolation(bounds, "")).toBeUndefined()
        expect(getAttributeLengthViolation(bounds, "   ")).toBeUndefined()
    })

    it("measures the same string Keycloak will", () => {
        // Trimmed by default, so the surrounding spaces are not characters.
        expect(getAttributeLengthViolation(bounds, "  ab  ")).toBeUndefined()

        const untrimmed = getAttributeLengthBounds(
            attribute({validations: {length: {"min": 2, "max": 4, "trim-disabled": "true"}}})
        )
        expect(getAttributeLengthViolation(untrimmed, "  ab  ")).toBe("tooLong")
    })

    it("bounds nothing when the attribute bounds nothing", () => {
        expect(getAttributeLengthViolation(undefined, "anything at all")).toBeUndefined()
    })
})

describe("getAttributeViolation", () => {
    const bounds = getAttributeLengthBounds(attribute({validations: {length: {min: 2, max: 3}}}))

    it("reports an empty required field as required, not as too short", () => {
        expect(getAttributeViolation(bounds, "", true)).toBe("required")
        expect(getAttributeViolation(bounds, "   ", true)).toBe("required")
    })

    it("leaves an empty optional field alone", () => {
        expect(getAttributeViolation(bounds, "", false)).toBeUndefined()
        expect(getAttributeViolation(bounds, "   ", false)).toBeUndefined()
    })

    it("still reports the bounds of a value that is there", () => {
        expect(getAttributeViolation(bounds, "a", true)).toBe("tooShort")
        expect(getAttributeViolation(bounds, "abcd", true)).toBe("tooLong")
        expect(getAttributeViolation(bounds, "ab", true)).toBeUndefined()
    })

    it("reports a required field with no bounds at all", () => {
        expect(getAttributeViolation(undefined, "", true)).toBe("required")
        expect(getAttributeViolation(undefined, "anything", true)).toBeUndefined()
    })

    it("counts an untrimmed attribute's spaces as a value", () => {
        const untrimmed = getAttributeLengthBounds(
            attribute({validations: {length: {"min": 2, "max": 3, "trim-disabled": "true"}}})
        )

        // The spaces are a value rather than an absent one, so this is three
        // characters, within the bounds, and not a required field left empty.
        expect(getAttributeViolation(untrimmed, "   ", true)).toBeUndefined()
        expect(getAttributeViolation(untrimmed, "    ", true)).toBe("tooLong")
    })
})

describe("isHiddenAttribute", () => {
    it("reads the flag Keycloak's admin console writes, as a string", () => {
        expect(isHiddenAttribute(attribute({annotations: {hidden: "true"}}))).toBe(true)
        expect(isHiddenAttribute(attribute({annotations: {hidden: "TRUE"}}))).toBe(true)
        expect(isHiddenAttribute(attribute({annotations: {hidden: " true "}}))).toBe(true)
    })

    it("reads the flag a realm configuration carries, as a boolean", () => {
        expect(isHiddenAttribute(attribute({annotations: {hidden: true}}))).toBe(true)
    })

    it("does not hide an attribute that is not marked", () => {
        expect(isHiddenAttribute(attribute({annotations: {hidden: "false"}}))).toBe(false)
        expect(isHiddenAttribute(attribute({annotations: {hidden: false}}))).toBe(false)
        expect(isHiddenAttribute(attribute({annotations: {hidden: ""}}))).toBe(false)
        expect(isHiddenAttribute(attribute({annotations: {}}))).toBe(false)
        expect(isHiddenAttribute(attribute({}))).toBe(false)
    })

    // inputType: "hidden" is Keycloak's own annotation for a hidden input and
    // means something else entirely.
    it("does not confuse the flag with a hidden input type", () => {
        expect(isHiddenAttribute(attribute({annotations: {inputType: "hidden"}}))).toBe(false)
    })
})

describe("getStatedLengthBounds", () => {
    const boundsOf = (length: Record<string, unknown>) =>
        getAttributeLengthBounds(attribute({validations: {length}}))

    it("states bounds a person actually types into", () => {
        expect(getStatedLengthBounds(boundsOf({min: 2, max: 3}))).toEqual({min: 2, max: 3})
        expect(getStatedLengthBounds(boundsOf({max: 2}))).toEqual({min: undefined, max: 2})
    })

    // Keycloak's base attributes carry these as scaffolding, and clients copy
    // them for free-text attributes; nobody types 255 characters into a name.
    it("does not state a maximum nobody reaches", () => {
        expect(getStatedLengthBounds(boundsOf({max: 255}))).toBeUndefined()
        expect(getStatedLengthBounds(boundsOf({min: 1, max: 255}))).toBeUndefined()
    })

    // A minimum of one says only that the value is present, which the field
    // already says by being required or not.
    it("does not state a minimum of one", () => {
        expect(getStatedLengthBounds(boundsOf({min: 1, max: 2}))).toEqual({
            min: undefined,
            max: 2,
        })
        expect(getStatedLengthBounds(boundsOf({min: 1}))).toBeUndefined()
    })

    it("still states a low maximum alongside a real minimum", () => {
        expect(getStatedLengthBounds(boundsOf({min: 2, max: 255}))).toEqual({
            min: 2,
            max: undefined,
        })
    })

    it("states nothing for an unbounded attribute", () => {
        expect(getStatedLengthBounds(undefined)).toBeUndefined()
    })
})
