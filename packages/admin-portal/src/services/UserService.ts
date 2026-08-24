// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {UserProfileAttribute} from "@/gql/graphql"

// Works for both camelCase and snake_case, i.e. in the format of ${attributeName} or ${profile.attributes.attributeName} or
// in snake case like ${first_name} or ${profile.attributes.first_name} and we want to convert it to a more readable format like 'First Name' or 'Personal Administrative Number'
// ${firstName} => First Name
// ${profile.attributes.firstName} => First Name
// ${profile.attributes.personal_administrative_number} => Personal Administrative Number
export const getAttributeLabel = (displayName: string) => {
    if (displayName?.includes("$")) {
        const rawName =
            displayName
                // Step 1: Remove '${' from the start and '}' from the end
                .replace(/^\${|}$/g, "")
                // Step 2: Remove any leading or trailing whitespace
                .trim()
                // Step 3: Get the word after the last dot if it exists, otherwise use the whole string
                .split(".")
                .pop() ?? ""
        return (
            rawName
                // Step 4 : Replace underscores with spaces
                .replace(/_/g, " ")
                // Step 5 : Add a space between a lowercase letter followed by an uppercase letter
                .replace(/([a-z])([A-Z])/g, "$1 $2")
                // Step 6: Capitalize the first letter and every letter after a space
                .replace(/\b\w/g, (match) => match.toUpperCase())
        )
    }
    return displayName ?? ""
}

export const getTranslationLabel = (
    name: string | undefined | null,
    displayName: string | undefined | null,
    t: (key: string) => string
) => {
    if (name) {
        const key = `usersAndRolesScreen.users.fields.${name}`
        const translated = t(key)
        if (translated !== key) {
            return translated
        }
    }
    return getAttributeLabel(displayName ?? "")
}

export const userBasicInfo = ["first_name", "last_name", "email", "username"]

const alwaysVisibleAdminAttributes = ["username"]

export const formatUserAtributes = (attributes: any) => {
    const newUserAttributesObject: Record<string, any> = {}
    if (attributes) {
        Object.entries(attributes).forEach(([key, value]) => {
            if (key !== "tenant-id") {
                newUserAttributesObject[`${key}`] = value
            }
        })
        return newUserAttributesObject
    }
    return null
}

const parseJson = (value: string): unknown => {
    try {
        return JSON.parse(value)
    } catch {
        return undefined
    }
}

/**
 * Descriptions Keycloak stores for the options of a `select` or
 * `multiselect-checkboxes` user profile attribute. A checkbox attribute takes
 * its options from this map's keys, so an entry whose description is unusable
 * keeps the option and describes it with itself, the way the login theme's
 * `inputOptionLabels[option]!option` default does.
 *
 * Keycloak's admin console writes annotation values as strings, so a map
 * configured there arrives JSON encoded while one written straight into the
 * realm configuration arrives as an object. Indexing the encoded form by an
 * option reads a character out of the JSON text rather than a description, so
 * it is decoded first.
 */
export const getInputOptionLabels = (
    attribute: UserProfileAttribute
): Record<string, string> | undefined => {
    const annotations = attribute.annotations as {inputOptionLabels?: unknown} | null | undefined
    const configured = annotations?.inputOptionLabels
    const decoded = typeof configured === "string" ? parseJson(configured) : configured
    if (typeof decoded !== "object" || decoded === null || Array.isArray(decoded)) {
        return undefined
    }

    const labels: Record<string, string> = {}
    Object.entries(decoded).forEach(([option, label]) => {
        labels[option] = typeof label === "string" && label.length > 0 ? label : option
    })

    return Object.keys(labels).length > 0 ? labels : undefined
}

/**
 * Text shown for one option description. Keycloak writes a description meant to
 * be localized as ${key}, the same convention display names use, so the key is
 * looked up before falling back to a readable form of it. Anything else is left
 * exactly as configured.
 */
export const resolveOptionLabel = (label: string, t: (key: string) => string): string => {
    const trimmed = label.trim()
    const placeholder = /^\$\{(.+)\}$/.exec(trimmed)
    if (!placeholder) {
        return trimmed
    }

    const translated = t(placeholder[1])

    return translated !== placeholder[1] ? translated : getAttributeLabel(trimmed)
}

/**
 * Description configured for one option of a `select` user profile attribute:
 * the label Keycloak stores for it, or a localization override of the option
 * itself, which is how this form has always let an option be given a readable
 * name. Returns undefined when the option is left undescribed.
 */
const getSelectOptionDescription = (
    optionLabels: Record<string, string> | undefined,
    option: string,
    t: (key: string) => string
): string | undefined => {
    const optionLabel = optionLabels?.[option]
    if (optionLabel && optionLabel !== option) {
        return resolveOptionLabel(optionLabel, t)
    }

    const translated = t(option)

    return translated !== option ? translated : undefined
}

/**
 * Label shown for one option of a `select` user profile attribute. A configured
 * description replaces the stored value; the value is the fallback when the
 * option has no description.
 */
export const getSelectOptionLabel = (
    optionLabels: Record<string, string> | undefined,
    option: string,
    t: (key: string) => string
): string => {
    const description = getSelectOptionDescription(optionLabels, option, t)

    return description ?? option
}

const toPositiveInteger = (value: unknown): number | undefined => {
    const parsed = typeof value === "string" ? Number(value) : value

    return typeof parsed === "number" && Number.isInteger(parsed) && parsed > 0 ? parsed : undefined
}

export interface AttributeLengthBounds {
    min?: number
    max?: number
    /** Whether the bounds are measured against the trimmed value. */
    trim: boolean
}

type AttributeLengthViolation = "tooShort" | "tooLong"

/**
 * The character bounds Keycloak's `length` validator puts on an attribute, or
 * undefined when it bounds nothing. Only the validator is read: the
 * `inputTypeMaxlength` annotation caps typing on the voter-facing forms but is
 * not enforced on submit, so stating it here would claim a rule the server does
 * not apply.
 */
export const getAttributeLengthBounds = (
    attribute: UserProfileAttribute
): AttributeLengthBounds | undefined => {
    const validations = attribute.validations as
        | {length?: {"min"?: unknown; "max"?: unknown; "trim-disabled"?: unknown} | null}
        | null
        | undefined
    // Narrowed before any bound is read off it: the jsonb can hold anything,
    // and a string or an array there would answer `.length` with a number.
    const length = validations?.length
    if (typeof length !== "object" || length === null || Array.isArray(length)) {
        return undefined
    }

    const min = toPositiveInteger(length.min)
    const max = toPositiveInteger(length.max)
    if (min === undefined && max === undefined) {
        return undefined
    }

    // Keycloak measures the trimmed value unless the attribute opts out, so the
    // form has to measure the same string the server will.
    const trimDisabled = length["trim-disabled"]

    return {min, max, trim: !(trimDisabled === true || trimDisabled === "true")}
}

/**
 * Which bound a value breaks, or undefined when it breaks none. An absent value
 * is a matter for the attribute being required, which the form states
 * separately, so a minimum only bounds a value that is actually there.
 */
const getAttributeLengthViolation = (
    bounds: AttributeLengthBounds | undefined,
    value: string
): AttributeLengthViolation | undefined => {
    if (!bounds) {
        return undefined
    }

    const measured = bounds.trim ? value.trim() : value
    if (measured.length === 0) {
        return undefined
    }
    if (bounds.min !== undefined && measured.length < bounds.min) {
        return "tooShort"
    }
    if (bounds.max !== undefined && measured.length > bounds.max) {
        return "tooLong"
    }

    return undefined
}

type AttributeViolation = AttributeLengthViolation | "required"

/**
 * What a touched field has to report, or undefined when it has nothing to
 * report. An attribute that is required and left empty reports that rather than
 * a bound, since an empty value breaks no bound.
 */
export const getAttributeViolation = (
    bounds: AttributeLengthBounds | undefined,
    value: string,
    required: boolean
): AttributeViolation | undefined => {
    const measured = bounds?.trim === false ? value : value.trim()
    if (required && measured.length === 0) {
        return "required"
    }

    return getAttributeLengthViolation(bounds, value)
}

/**
 * Whether an attribute should be hidden in the admin portal.
 *
 * This is Sequent's own annotation rather than a Keycloak one: Keycloak stores
 * and returns it without acting on it, and the login theme reads it to keep the
 * attribute off the voter-facing forms. Username is the exception: hiding it at
 * login must not remove Keycloak's built-in identifier from admin lists and
 * forms. Other attributes are read exactly as the theme reads them: the theme
 * matches the literal string, which is what Keycloak's admin console writes,
 * and a realm configuration can carry the boolean instead.
 */
export const isHiddenAttribute = (attribute: UserProfileAttribute): boolean => {
    if (attribute.name && alwaysVisibleAdminAttributes.includes(attribute.name)) {
        return false
    }

    const annotations = attribute.annotations as {hidden?: unknown} | null | undefined
    const hidden = annotations?.hidden

    return hidden === true || hidden === "true"
}

/**
 * The largest maximum worth stating under a field. Keycloak's own base
 * attributes carry maxima in the hundreds as scaffolding rather than as a rule
 * anyone types into, and clients tend to copy that for free-text attributes, so
 * stating them would put a hint under most of a voter form for a bound nobody
 * reaches.
 */
const STATED_MAXIMUM_LENGTH = 100

export interface StatedLengthBounds {
    min?: number
    max?: number
}

/**
 * The bounds worth stating under a field, or undefined when none of them are.
 * The field is still checked against everything the validator sets: this only
 * decides what is worth saying up front.
 */
export const getStatedLengthBounds = (
    bounds: AttributeLengthBounds | undefined
): StatedLengthBounds | undefined => {
    if (!bounds) {
        return undefined
    }

    // A minimum of one says only that the value is present, which the field
    // already says by being required or not.
    const min = bounds.min !== undefined && bounds.min > 1 ? bounds.min : undefined
    const max =
        bounds.max !== undefined && bounds.max <= STATED_MAXIMUM_LENGTH ? bounds.max : undefined

    return min === undefined && max === undefined ? undefined : {min, max}
}
