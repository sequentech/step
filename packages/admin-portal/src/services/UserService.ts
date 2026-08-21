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
export const getSelectOptionDescription = (
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
 * Label shown for one option of a `select` user profile attribute. The stored
 * option stays visible, so the admin still sees what is written to the voter,
 * and its description is appended when the attribute configures one.
 */
export const getSelectOptionLabel = (
    optionLabels: Record<string, string> | undefined,
    option: string,
    t: (key: string) => string
): string => {
    const description = getSelectOptionDescription(optionLabels, option, t)

    return description && description !== option ? `${option} - ${description}` : option
}
