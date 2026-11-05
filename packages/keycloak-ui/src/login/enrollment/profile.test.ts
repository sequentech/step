// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import type {Attribute} from "keycloakify/login/KcContext"
import {CredentialFieldPosition} from "../KcContext"
import {
    StrengthLevel,
    credentialPlacement,
    fieldToggles,
    filteredOptions,
    htmlAttributes,
    initialValues,
    inputTypeOf,
    optionsOf,
    strengthOf,
    timezoneCountry,
    visibleAttributes,
} from "./profile"

function attribute(name: string, extra: Record<string, unknown> = {}): Attribute {
    return {
        name,
        required: false,
        readOnly: false,
        validators: {},
        annotations: {},
        ...extra,
    } as Attribute
}

describe("visibleAttributes", () => {
    it("leaves out the attributes the authenticator hides", () => {
        const profile = [attribute("username"), attribute("email"), attribute("embassy")]
        expect(visibleAttributes(profile, ["email"]).map(({name}) => name)).toEqual([
            "username",
            "embassy",
        ])
    })
})

describe("credentialPlacement", () => {
    const place = (attributes: Attribute[], overrides = {}) =>
        credentialPlacement({
            attributes,
            emailAsUsername: false,
            passwordRequired: true,
            position: CredentialFieldPosition.Last,
            ...overrides,
        })

    it("puts the password after the username", () => {
        const placement = place([attribute("firstName"), attribute("username")])
        expect(placement).toMatchObject({first: false, anchor: "username"})
        expect(placement.source.name).toBe("username")
    })

    it("puts it after the email when the email is the username", () => {
        const profile = [attribute("username"), attribute("email")]
        expect(place(profile, {emailAsUsername: true}).anchor).toBe("email")
    })

    it("prefers the attribute that asks for the password after it", () => {
        const profile = [
            attribute("username"),
            attribute("embassy", {annotations: {showPasswordAfterThis: "true"}}),
        ]
        const placement = place(profile, {position: CredentialFieldPosition.First})
        // The annotation always wins over the realm's position.
        expect(placement).toMatchObject({first: false, anchor: "embassy"})
        expect(placement.source.name).toBe("embassy")
    })

    it("has no place for it when the username opts out", () => {
        const profile = [attribute("username", {annotations: {showPasswordAfterThis: "false"}})]
        expect(place(profile)).toMatchObject({first: false, anchor: ""})
    })

    it("puts it first when the realm asks for it", () => {
        const profile = [attribute("username")]
        expect(place(profile, {position: CredentialFieldPosition.First}).first).toBe(true)
        expect(
            place(profile, {position: CredentialFieldPosition.First, passwordRequired: false}).first
        ).toBe(false)
    })

    it("reads the password's helper texts from the first attribute without a username", () => {
        expect(place([attribute("embassy")]).source.name).toBe("embassy")
        expect(place([])).toMatchObject({anchor: "", source: {name: "password"}})
    })
})

describe("field values and types", () => {
    it("starts from the submitted values, then the default annotation", () => {
        expect(initialValues(attribute("a", {values: ["x", "y"]}))).toEqual(["x", "y"])
        expect(initialValues(attribute("a", {value: "x"}))).toEqual(["x"])
        expect(initialValues(attribute("a", {annotations: {default: "z"}}))).toEqual(["z"])
        expect(initialValues(attribute("a", {values: [], annotations: {default: ""}}))).toEqual([])
    })

    it("maps the inputType annotation to an input type", () => {
        expect(inputTypeOf(attribute("a"))).toBe("text")
        expect(inputTypeOf(attribute("a", {annotations: {inputType: "html5-tel"}}))).toBe("tel")
        expect(inputTypeOf(attribute("a", {annotations: {inputType: "select"}}))).toBe("select")
    })

    it("reads the options from the validator the attribute names", () => {
        const options = {options: ["red", "green"]}
        expect(optionsOf(attribute("a", {validators: {options}}))).toEqual(["red", "green"])
        expect(
            optionsOf(
                attribute("a", {
                    annotations: {inputOptionsFromValidation: "colours"},
                    validators: {colours: {options: ["blue"]}, options},
                })
            )
        ).toEqual(["blue"])
        expect(optionsOf(attribute("a"))).toEqual([])
    })

    it("collects the html-attribute annotations", () => {
        expect(
            htmlAttributes(
                attribute("a", {
                    annotations: {
                        "html-attribute:autocomplete": "off",
                        "html-attribute:style": "text-transform: uppercase",
                        inputType: "text",
                    },
                })
            )
        ).toEqual({autocomplete: "off", style: "text-transform: uppercase"})
    })

    it("narrows a select to the options that contain the chosen value", () => {
        expect(filteredOptions(["PH-manila", "PH-cebu", "ES-madrid"], "PH")).toEqual([
            "PH-manila",
            "PH-cebu",
        ])
        expect(filteredOptions(["a", "b"], undefined)).toEqual(["a", "b"])
    })
})

describe("fieldToggles", () => {
    const contact = (annotations: Record<string, string>) =>
        attribute("contact", {
            annotations: {inputType: "multiselect-checkboxes", ...annotations},
            validators: {options: {options: ["email", "phone"]}},
        })

    it("locks and stops requiring the fields of the unchecked options", () => {
        const toggles = fieldToggles([contact({disableAttribute: "true"})], {contact: ["email"]})
        expect(toggles.email).toEqual({readOnly: false, required: true})
        expect(toggles.phone).toEqual({readOnly: true, required: false})
        // The phone widget's visible input and the confirmation follow them.
        expect(toggles["phone-input"]).toEqual(toggles.phone)
        expect(toggles["email-confirm"]).toEqual(toggles.email)
    })

    it("disables the elements of the unchecked options", () => {
        const toggles = fieldToggles([contact({disableElement: "true"})], {contact: []})
        expect(toggles.email).toEqual({disabled: true})
        expect(toggles["email-confirm"]).toBeUndefined()
    })

    it("leaves other attributes alone", () => {
        expect(fieldToggles([contact({})], {contact: []})).toEqual({})
    })
})

describe("strengthOf", () => {
    it("has no strength for an empty password", () => {
        expect(strengthOf("", 0)).toEqual({percent: 0, level: StrengthLevel.None})
    })

    it("grows with the score", () => {
        expect(strengthOf("a", 0)).toEqual({percent: 20, level: StrengthLevel.Weak})
        expect(strengthOf("a", 2)).toEqual({percent: 60, level: StrengthLevel.Fair})
        expect(strengthOf("a", 4)).toEqual({percent: 100, level: StrengthLevel.Strong})
    })
})

describe("timezoneCountry", () => {
    const data = '{"Asia/Manila":["PH"],"Europe/Madrid":["ES"]}'

    it("finds the country of the browser's time zone", () => {
        expect(timezoneCountry(data, "Asia/Manila")).toBe("PH")
    })

    it("has none for unknown zones or unreadable data", () => {
        expect(timezoneCountry(data, "Mars/Olympus")).toBeUndefined()
        expect(timezoneCountry("not json", "Asia/Manila")).toBeUndefined()
        expect(timezoneCountry(undefined, "Asia/Manila")).toBeUndefined()
    })
})
