// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    ETransparencyMode,
    IVitrineLogLocation,
    IVitrineLoggedStatement,
    IVitrinePresentation,
    IVitrineVoterContext,
} from "./types"

type JsonObject = Record<string, unknown>

const POLICY_ID_PREFIX = "sha256:"
const POLICY_HEX = /^[0-9a-f]{64}$/

/** A jsonb value as an object: Hasura returns jsonb as an object, and older
 * clients or caches may hold it as its JSON text. */
export const asObject = (value: unknown): JsonObject | undefined => {
    let v = value
    if (typeof v === "string") {
        try {
            v = JSON.parse(v)
        } catch {
            return undefined
        }
    }
    return v !== null && typeof v === "object" && !Array.isArray(v) ? (v as JsonObject) : undefined
}

const asString = (value: unknown): string | undefined =>
    typeof value === "string" && value !== "" ? value : undefined

const readLog = (value: unknown): IVitrineLogLocation | undefined => {
    const o = asObject(value)
    const url = asString(o?.url)
    const origin = asString(o?.origin)
    return url && origin ? {url, origin} : undefined
}

/** The lowercase hex of a policy ID, given with or without `sha256:`. */
export const policyHex = (id: string | undefined): string | undefined => {
    if (!id) return undefined
    const hex = id.startsWith(POLICY_ID_PREFIX) ? id.slice(POLICY_ID_PREFIX.length) : id
    return POLICY_HEX.test(hex) ? hex : undefined
}

export interface ITransparency {
    mode: ETransparencyMode
    vitrine?: IVitrinePresentation
}

const readMode = (value: unknown): ETransparencyMode =>
    value === ETransparencyMode.PUBLIC || value === ETransparencyMode.PRIVATE
        ? value
        : ETransparencyMode.DISABLED

/** The transparency settings of an election event's `presentation`. An absent
 * or unknown mode is `disabled`, as for events created before Vitrine. */
export const readTransparency = (presentation: unknown): ITransparency => {
    const p = asObject(presentation)
    const mode = readMode(p?.transparency)
    if (mode === ETransparencyMode.DISABLED) return {mode}
    const v = asObject(p?.vitrine)
    const vitrine: IVitrinePresentation = {}
    const id = asString(v?.policy_id)
    if (id) vitrine.policy_id = id
    const policy = asString(v?.policy)
    if (policy) vitrine.policy = policy
    const logs = asObject(v?.logs)
    const ballots = readLog(logs?.ballots)
    const events = readLog(logs?.events)
    if (ballots || events) {
        vitrine.logs = {}
        if (ballots) vitrine.logs.ballots = ballots
        if (events) vitrine.logs.events = events
    }
    return {mode, vitrine}
}

/** Transparency settings kept by the portal itself (across the golden
 * re-authentication reload), read as defensively as a presentation. */
export const asTransparency = (value: unknown): ITransparency => {
    const o = asObject(value)
    return readTransparency({transparency: o?.mode, vitrine: o?.vitrine})
}

/** The first presentation that says anything about transparency: the event's
 * own, then the copy in the ballot style. */
export const eventTransparency = (...presentations: unknown[]): ITransparency => {
    for (const p of presentations) {
        const o = asObject(p)
        if (o && "transparency" in o) return readTransparency(o)
    }
    return {mode: ETransparencyMode.DISABLED}
}

const readStatement = (value: unknown): IVitrineLoggedStatement | null => {
    const o = asObject(value)
    const statement = asString(o?.statement)
    const entry = asString(o?.entry)
    const chain = asString(o?.chain)
    return statement && entry && chain ? {statement, entry, chain} : null
}

/** `vitrine_voter_context` with its jsonb fields read defensively: anything
 * malformed reads as absent, which the booth check refuses. */
export const readVoterContext = (value: unknown): IVitrineVoterContext | undefined => {
    const o = asObject(value)
    const mode = asString(o?.mode)
    if (!o || !mode) return undefined
    const hashes = Array.isArray(o.style_hashes)
        ? o.style_hashes.filter((h): h is string => typeof h === "string")
        : null
    return {
        mode,
        policy: asString(o.policy) ?? null,
        policy_ack: asString(o.policy_ack) ?? null,
        config: readStatement(o.config),
        opened: readStatement(o.opened),
        style_hashes: hashes,
        release: asString(o.release) ?? null,
    }
}
