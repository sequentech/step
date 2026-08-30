// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

const MARKER_START = "\u2060"
const BIT_ZERO = "\u200B"
const BIT_ONE = "\u200C"

const WRAP_PATTERN = /\u2060([\u200B\u200C]+)\u2060/g

const keyByIndex: string[] = []
const indexByKey = new Map<string, number>()

const registerKey = (key: string): number => {
    const existing = indexByKey.get(key)
    if (existing !== undefined) {
        return existing
    }
    const index = keyByIndex.length
    keyByIndex.push(key)
    indexByKey.set(key, index)
    return index
}

const encodeIndex = (index: number): string => {
    const bits = index.toString(2)
    return `${MARKER_START}${bits.replace(/0/g, BIT_ZERO).replace(/1/g, BIT_ONE)}${MARKER_START}`
}

const decodeIndex = (bits: string): number => {
    const binary = bits.replace(new RegExp(BIT_ZERO, "g"), "0").replace(new RegExp(BIT_ONE, "g"), "1")
    return Number.parseInt(binary, 2)
}

export const wrapTranslation = (key: string, value: string): string => {
    if (!key || !value || value === key || value.includes(MARKER_START)) {
        return value
    }
    return `${encodeIndex(registerKey(key))}${value}`
}

export const stripMarkers = (value: string): string => value.replace(WRAP_PATTERN, "")

export const keysFromText = (value: string): string[] => {
    const keys: string[] = []
    WRAP_PATTERN.lastIndex = 0
    let match = WRAP_PATTERN.exec(value)
    while (match) {
        const index = decodeIndex(match[1])
        const key = keyByIndex[index]
        if (key) {
            keys.push(key)
        }
        match = WRAP_PATTERN.exec(value)
    }
    return keys
}

export const firstKeyFromText = (value: string): string | null => keysFromText(value)[0] ?? null
