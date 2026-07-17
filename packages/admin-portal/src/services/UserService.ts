// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

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
