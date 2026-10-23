// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The two statements the booth reads before it encrypts (specs/entry.md,
// sections 5.2 and 5.3): a header line, then `<key> <value>` lines in the
// order the spec gives, each at most once unless it is repeatable.

export const CONFIG_HEADER = "vitrine/config/v1"
export const MILESTONE_HEADER = "vitrine/milestone/v1"
export const POLICY_ACK_HEADER = "vitrine/policy-ack/v1"
export const VOTING_OPENED = "voting-opened"

const CONFIG_KEYS = [
    "event",
    "election",
    "key",
    "styles",
    "opens",
    "closes",
    "policy",
    "release",
    "transparency",
    "demo",
    "time",
] as const
const CONFIG_REPEATABLE = ["release"]
const CONFIG_REQUIRED = CONFIG_KEYS.filter((key) => key !== "release")

const MILESTONE_KEYS = ["event", "kind", "time", "config", "count"] as const
const MILESTONE_REPEATABLE = ["config", "count"]

const POLICY_ACK_KEYS = ["origin", "policy", "time"] as const

type Lines = Map<string, string[]>

const parseLines = (
    text: string,
    header: string,
    order: readonly string[],
    repeatable: readonly string[]
): Lines | undefined => {
    if (!text.endsWith("\n")) return undefined
    const lines = text.slice(0, -1).split("\n")
    if (lines[0] !== header) return undefined
    const values: Lines = new Map()
    let at = 0
    for (const line of lines.slice(1)) {
        const space = line.indexOf(" ")
        if (space <= 0 || space === line.length - 1) return undefined
        const key = line.slice(0, space)
        const position = order.indexOf(key)
        if (position < at) return undefined
        if (values.has(key) && !repeatable.includes(key)) return undefined
        at = position
        values.set(key, [...(values.get(key) ?? []), line.slice(space + 1)])
    }
    return values
}

const one = (lines: Lines, key: string): string => lines.get(key)?.[0] ?? ""

export interface IRelease {
    package: string
    version: string
}

export interface IConfigStatement {
    event: string
    election: string
    key: string
    styles: string
    opens: string
    closes: string
    policy: string
    releases: IRelease[]
    transparency: string
    demo: string
    time: string
}

/** A `vitrine/config/v1` statement's text, or undefined when malformed. */
export const parseConfigStatement = (text: string): IConfigStatement | undefined => {
    const lines = parseLines(text, CONFIG_HEADER, CONFIG_KEYS, CONFIG_REPEATABLE)
    if (!lines || CONFIG_REQUIRED.some((key) => !lines.has(key))) return undefined
    const releases: IRelease[] = []
    for (const value of lines.get("release") ?? []) {
        const parts = value.split(" ")
        if (parts.length !== 2 || !parts[0] || !parts[1]) return undefined
        releases.push({package: parts[0], version: parts[1]})
    }
    return {
        event: one(lines, "event"),
        election: one(lines, "election"),
        key: one(lines, "key"),
        styles: one(lines, "styles"),
        opens: one(lines, "opens"),
        closes: one(lines, "closes"),
        policy: one(lines, "policy"),
        releases,
        transparency: one(lines, "transparency"),
        demo: one(lines, "demo"),
        time: one(lines, "time"),
    }
}

export interface IMilestoneStatement {
    event: string
    kind: string
    time: string
    /** For `voting-opened`: the leaf hash of each election's config entry
     * (base64), by election ID. */
    configs: Record<string, string>
}

/** A `vitrine/milestone/v1` statement's text, or undefined when malformed. */
export const parseMilestoneStatement = (text: string): IMilestoneStatement | undefined => {
    const lines = parseLines(text, MILESTONE_HEADER, MILESTONE_KEYS, MILESTONE_REPEATABLE)
    if (!lines || !lines.has("event") || !lines.has("kind") || !lines.has("time")) {
        return undefined
    }
    const configs: Record<string, string> = {}
    for (const value of lines.get("config") ?? []) {
        const parts = value.split(" ")
        if (parts.length !== 2 || !parts[0] || !parts[1]) return undefined
        if (Object.prototype.hasOwnProperty.call(configs, parts[0])) return undefined
        configs[parts[0]] = parts[1]
    }
    return {event: one(lines, "event"), kind: one(lines, "kind"), time: one(lines, "time"), configs}
}

export interface IPolicyAck {
    origin: string
    policy: string
    time: string
}

/** A `vitrine/policy-ack/v1` note's text (specs/entry.md, section 5.7). */
export const parsePolicyAck = (text: string): IPolicyAck | undefined => {
    const lines = parseLines(text, POLICY_ACK_HEADER, POLICY_ACK_KEYS, [])
    if (!lines || POLICY_ACK_KEYS.some((key) => !lines.has(key))) return undefined
    return {origin: one(lines, "origin"), policy: one(lines, "policy"), time: one(lines, "time")}
}
