// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
export const getAttributeLabel = (displayName: string) => {
    if (displayName?.includes("$")) {
        return (
            displayName
                // Step 1: Remove '${' from the start and '}' from the end
                .replace(/^\${|}$/g, "")
                // Step 2: Remove any leading or trailing whitespace
                .trim()
                // Step 3: Add a space between a lowercase letter followed by an uppercase letter
                .replace(/([a-z])([A-Z])/g, "$1 $2")
                // Step 4: Capitalize the first letter
                .replace(/^./, (match) => match.toUpperCase()) ?? ""
        )
    }
    return displayName ?? ""
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
