// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
export const getAttributeLabel = (displayName: string) => {
    if (displayName?.includes("$")) {
        return (
            displayName
                .replace(/^\${|}$/g, "")
                .trim()
                .replace(/([a-z])([A-Z])/g, "$1 $2")
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
