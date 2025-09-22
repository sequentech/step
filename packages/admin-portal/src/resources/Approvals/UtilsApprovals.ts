// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Define the input and output object types
type CamelCaseObject = Record<string, any>
type SnakeCaseObject = Record<string, any>

export function convertToSnakeCase(obj: CamelCaseObject): SnakeCaseObject {
    const newObj: SnakeCaseObject = {}

    Object.keys(obj).forEach((key) => {
        const snakeKey = key.replace(/([A-Z])/g, "_$1").toLowerCase()
        newObj[snakeKey] = obj[key]
    })

    return newObj
}

export function convertOneToSnakeCase(key: string): string {
    return key.replace(/([A-Z])/g, "_$1").toLowerCase()
}

export function convertToCamelCase(input: string): string {
    return input.replace(/_([a-z])/g, (match, letter) => letter.toUpperCase())
}
