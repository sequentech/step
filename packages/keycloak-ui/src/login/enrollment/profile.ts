// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Attribute} from "keycloakify/login/KcContext"
import {CredentialFieldPosition} from "../KcContext"

// The registration form's rules, as sequent-theme's register.ftl and
// user-profile-commons.ftl apply them.

// Sequent's annotations aren't in Keycloakify's types.
export function annotation(attribute: Attribute, key: string): string | undefined {
    const value = (attribute.annotations as Record<string, unknown>)[key]
    return value === undefined || value === null ? undefined : String(value)
}

export function visibleAttributes(attributes: Attribute[], hidden: string[]): Attribute[] {
    return attributes.filter(({name}) => !hidden.includes(name))
}

export type CredentialPlacement = {
    // Above the profile fields.
    first: boolean
    // The attribute the password goes after; none when empty.
    anchor: string
    // The attribute whose annotations describe the password field.
    source: Attribute
}

const NO_SOURCE = {name: "password", annotations: {}} as Attribute
const SHOW_PASSWORD_AFTER = "showPasswordAfterThis"

export function credentialPlacement(params: {
    attributes: Attribute[]
    emailAsUsername: boolean
    passwordRequired: boolean
    position: CredentialFieldPosition
}): CredentialPlacement {
    const {attributes, emailAsUsername, passwordRequired, position} = params
    const usernameAttribute = emailAsUsername ? "email" : "username"
    const byDefault = attributes.find(({name}) => name === usernameAttribute)
    const explicit = attributes.find((item) => annotation(item, SHOW_PASSWORD_AFTER) === "true")
    const anyExplicit = attributes.some(
        (item) => annotation(item, SHOW_PASSWORD_AFTER) !== undefined
    )
    const source = explicit ?? byDefault ?? attributes[0] ?? NO_SOURCE
    const anchor =
        explicit !== undefined
            ? explicit.name
            : byDefault !== undefined && annotation(byDefault, SHOW_PASSWORD_AFTER) !== "false"
              ? byDefault.name
              : ""
    return {
        first: passwordRequired && position === CredentialFieldPosition.First && !anyExplicit,
        anchor,
        source,
    }
}

export function initialValues(attribute: Attribute): string[] {
    if (attribute.values !== undefined && attribute.values.length > 0) return attribute.values
    if (attribute.value !== undefined && attribute.value !== "") return [attribute.value]
    const byDefault = annotation(attribute, "default")
    return byDefault === undefined || byDefault === "" ? [] : [byDefault]
}

const HTML5_PREFIX = "html5-"

export function inputTypeOf(attribute: Attribute): string {
    const type = attribute.annotations.inputType
    if (type === undefined || type === "") return "text"
    return type.startsWith(HTML5_PREFIX) ? type.slice(HTML5_PREFIX.length) : type
}

export function optionsOf(attribute: Attribute): string[] {
    const validators = attribute.validators as Record<string, {options?: string[]} | undefined>
    const named = attribute.annotations.inputOptionsFromValidation
    return (
        (named === undefined ? undefined : validators[named]?.options) ??
        validators.options?.options ??
        []
    )
}

const HTML_ATTRIBUTE_PREFIX = "html-attribute:"

export function htmlAttributes(attribute: Attribute): Record<string, string> {
    return Object.fromEntries(
        Object.entries(attribute.annotations as Record<string, unknown>)
            .filter(([key]) => key.startsWith(HTML_ATTRIBUTE_PREFIX))
            .map(([key, value]) => [key.slice(HTML_ATTRIBUTE_PREFIX.length), String(value)])
    )
}

export function filteredOptions(options: string[], filter: string | undefined): string[] {
    return filter === undefined ? options : options.filter((option) => option.includes(filter))
}

export type FieldToggle = {readOnly?: boolean; required?: boolean; disabled?: boolean}

// The options of an attribute with disableAttribute or disableElement name the
// fields they switch: unchecked, the field is locked or disabled.
export function fieldToggles(
    attributes: Attribute[],
    values: Record<string, string[]>
): Record<string, FieldToggle> {
    const toggles: Record<string, FieldToggle> = {}
    for (const attribute of attributes) {
        const locks = annotation(attribute, "disableAttribute") !== undefined
        const disables = annotation(attribute, "disableElement") !== undefined
        if (!locks && !disables) continue
        for (const option of optionsOf(attribute)) {
            const checked = (values[attribute.name] ?? []).includes(option)
            if (locks) {
                const toggle = {readOnly: !checked, required: checked}
                for (const id of [option, `${option}-input`, `${option}-confirm`]) {
                    toggles[id] = {...toggles[id], ...toggle}
                }
            }
            if (disables) toggles[option] = {...toggles[option], disabled: !checked}
        }
    }
    return toggles
}

export enum StrengthLevel {
    None = "NONE",
    Weak = "WEAK",
    Fair = "FAIR",
    Strong = "STRONG",
}

const LEVELS = [
    StrengthLevel.Weak,
    StrengthLevel.Fair,
    StrengthLevel.Fair,
    StrengthLevel.Strong,
    StrengthLevel.Strong,
]

// From zxcvbn's score, 0 to 4.
export function strengthOf(
    password: string,
    score: number
): {percent: number; level: StrengthLevel} {
    if (password === "") return {percent: 0, level: StrengthLevel.None}
    return {percent: (score + 1) * 20, level: LEVELS[score] ?? StrengthLevel.Weak}
}

// The theme's map of time zones to countries, for the phone prefix.
export function timezoneCountry(data: string | undefined, timeZone: string): string | undefined {
    if (data === undefined) return undefined
    try {
        const countries = (JSON.parse(data) as Record<string, string[] | undefined>)[timeZone]
        return countries?.[0]
    } catch {
        return undefined
    }
}
