// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// How the Transparency tab reads Harvest's TransparencyOutput
// (step-integration.md, section 2.8). All of it is data: the sidecar's JSON is
// read field by field, and a policy's ID and words come from @vitrine/verify,
// never from the server.

import type * as VitrineVerify from "@vitrine/verify"
import type {Policy, PolicySummary, Reason} from "@vitrine/verify"
import type {Language} from "@vitrine/ui"

export type TVitrineVerify = typeof VitrineVerify

export enum ETransparencyMode {
    DISABLED = "disabled",
    PUBLIC = "public",
    PRIVATE = "private",
}

/** The modes an admin can enable. */
export const ENABLED_MODES: readonly ETransparencyMode[] = [
    ETransparencyMode.PUBLIC,
    ETransparencyMode.PRIVATE,
]

/** Why Harvest refuses `public`. */
export enum EPublicRefusal {
    /** A public ballots log shows that a ballot was cast, so voters must be
     * able to vote again as often as they need: every election allows
     * unlimited revotes. */
    UNLIMITED_REVOTES = "public-needs-unlimited-revotes",
}

/** An absent or unknown mode reads as `disabled`, as for events created
 * before Vitrine. */
export const readMode = (value: unknown): ETransparencyMode =>
    value === ETransparencyMode.PUBLIC || value === ETransparencyMode.PRIVATE
        ? value
        : ETransparencyMode.DISABLED

export const isKnownRefusal = (code: string): code is EPublicRefusal =>
    (Object.values(EPublicRefusal) as string[]).includes(code)

// -- Languages ----------------------------------------------------------------

/** The languages the Vitrine UI kit ships. */
const KIT_LANGUAGES: readonly Language[] = ["en", "ca", "es", "eu", "fr", "gl", "nl", "tl"]

/** The admin portal's codes that differ from the kit's. */
const APP_TO_KIT: Record<string, Language> = {cat: "ca"}

const isKitLanguage = (lang: string): lang is Language =>
    (KIT_LANGUAGES as readonly string[]).includes(lang)

/** The UI kit's language for one of the portal's: the same code, except the
 * portal's `cat`, which is the kit's `ca`. Anything else reads as English. */
export const vitrineLanguage = (appLanguage: string | undefined): Language => {
    const base = (appLanguage ?? "").toLowerCase().split(/[-_]/)[0] ?? ""
    const lang = APP_TO_KIT[base] ?? base
    return isKitLanguage(lang) ? lang : "en"
}

// -- Trust policies -------------------------------------------------------------

const POLICY_ID_PREFIX = "sha256:"
const POLICY_HEX = /^[0-9a-f]{64}$/
const BALLOTS_LOG = "ballots"
const EVENT_ID_PARAM = "event-id"

/** The sidecar's limit for a note, which a signed policy is. */
export const MAX_POLICY_CHARS = 64 * 1024

export enum EPolicyState {
    /** Sealed and signed: a policy the logs can operate under. */
    SEALED = "sealed",
    /** An unsealed draft, which is never valid. */
    DRAFT = "draft",
    INVALID = "invalid",
}

export type TPolicyExamination =
    | {state: EPolicyState.SEALED; policy: Policy; summary: PolicySummary}
    | {state: EPolicyState.DRAFT; policy: Policy; words: string[]}
    | {state: EPolicyState.INVALID; reason: Reason}

/** What `text` is: a valid sealed policy with its summary, a draft with its
 * eight words, or neither, with the verifier's reason. */
export const examinePolicy = (verify: TVitrineVerify, text: string): TPolicyExamination => {
    try {
        const policy = verify.pinPolicy(text)
        return {state: EPolicyState.SEALED, policy, summary: verify.policySummary(policy)}
    } catch (e) {
        try {
            const draft = verify.parsePolicy(text, {allowDraft: true, requireSignatures: false})
            if (draft.draft) {
                return {state: EPolicyState.DRAFT, policy: draft, words: verify.policyWords(draft)}
            }
        } catch {
            // Not a draft either: the first error says why.
        }
        return {
            state: EPolicyState.INVALID,
            reason: e instanceof verify.VerifyError ? e.toReason() : {code: "policy-invalid"},
        }
    }
}

/** A pasted or uploaded policy as the logs take it: `\n` line ends, nothing
 * around it, and the final newline a paste may have lost. Empty when there is
 * nothing but whitespace. */
export const normalisePolicyText = (raw: string): string => {
    const text = raw.replace(/\r\n/g, "\n").trim()
    return text === "" ? "" : `${text}\n`
}

/** A policy ID's lowercase hex, given with or without `sha256:`. */
export const policyHex = (id: unknown): string | undefined => {
    if (typeof id !== "string") return undefined
    const lower = id.toLowerCase()
    const hex = lower.startsWith(POLICY_ID_PREFIX) ? lower.slice(POLICY_ID_PREFIX.length) : lower
    return POLICY_HEX.test(hex) ? hex : undefined
}

export enum EPolicyMismatch {
    /** Its ballots log names another event, or none. */
    EVENT = "event",
    /** It is public for a private event, or private for a public one. */
    VISIBILITY = "visibility",
}

/** Why a sealed policy can't be this event's, as Harvest also checks. */
export const policyMismatch = (
    policy: Policy,
    electionEventId: string,
    mode: ETransparencyMode
): EPolicyMismatch | undefined => {
    const eventId = policy.logs.get(BALLOTS_LOG)?.params.get(EVENT_ID_PARAM)?.[0]
    if (eventId?.toLowerCase() !== electionEventId.toLowerCase()) return EPolicyMismatch.EVENT
    if (policy.visibility !== mode) return EPolicyMismatch.VISIBILITY
    return undefined
}

/** True when both are the same draft, sealed or not: a sealed policy keeps
 * the draft hash of the draft it was sealed from. */
export const sameDraft = (a: Policy, b: Policy): boolean =>
    a.draftHash.length === b.draftHash.length && a.draftHash.every((x, i) => x === b.draftHash[i])

export const draftFileName = (electionEventId: string): string =>
    `vitrine-policy-draft-${electionEventId}.txt`

/** The Board's commands for the draft (Vitrine's docs/operations.md, section
 * 10.2): check it, compare its words with the tab's, seal and sign it, and
 * have any other authority the `sign` line needs sign it too. */
export const boardCommands = (electionEventId: string): string[] => {
    const draft = draftFileName(electionEventId)
    const sealed = `vitrine-policy-${electionEventId}.txt`
    const signed = `vitrine-policy-${electionEventId}-signed.txt`
    return [
        `vitrine policy check ${draft}`,
        `vitrine policy words ${draft}`,
        `vitrine policy seal ${draft} --key <board key file> --out ${sealed}`,
        `vitrine policy sign ${sealed} --key <authority key file> --out ${signed}`,
    ]
}

// -- Logs, witnesses and hubs -------------------------------------------------------

type TJson = Record<string, unknown>

const asObject = (value: unknown): TJson | undefined => {
    let v = value
    if (typeof v === "string") {
        try {
            v = JSON.parse(v)
        } catch {
            return undefined
        }
    }
    return v !== null && typeof v === "object" && !Array.isArray(v) ? (v as TJson) : undefined
}

const asArray = (value: unknown): unknown[] => {
    let v = value
    if (typeof v === "string") {
        try {
            v = JSON.parse(v)
        } catch {
            return []
        }
    }
    return Array.isArray(v) ? v : []
}

const asString = (value: unknown): string | undefined =>
    typeof value === "string" && value !== "" ? value : undefined

const asNumber = (value: unknown): number | undefined =>
    typeof value === "number" && Number.isFinite(value) ? value : undefined

export interface ICheckpointView {
    size: number
    /** When it met the quorum, in POSIX seconds. */
    time?: number
}

export interface IWitnessView {
    name: string
    /** It cosigned the checkpoint the log serves. */
    current: boolean
    /** Entries between its latest cosignature and the served checkpoint. */
    behind?: number
    cosignedAgeSeconds?: number
    lastError?: string
    alert: boolean
}

export interface IHubAnchor {
    size?: number
    index?: number
    time?: number
}

export interface IHubView {
    name: string
    anchor?: IHubAnchor
}

export interface IQueueView {
    rows: number
    oldestSeconds?: number
}

export interface ILogView {
    /** The event's name for it (`ballots`, `events`), or the sidecar's. */
    key: string
    origin?: string
    visibility?: string
    size?: number
    checkpoint?: ICheckpointView
    /** The policy the log operates under. */
    policyHex?: string
    frozen: boolean
    alert?: string
    queue?: IQueueView
    witnesses: IWitnessView[]
    hubs: IHubView[]
}

const LOG_ORDER = ["ballots", "events"]

const byLogOrder = (a: string, b: string): number => {
    const rank = (key: string) => {
        const i = LOG_ORDER.indexOf(key)
        return i < 0 ? LOG_ORDER.length : i
    }
    return rank(a) - rank(b) || a.localeCompare(b)
}

const readCheckpoint = (value: unknown): ICheckpointView | undefined => {
    const o = asObject(value)
    const size = asNumber(o?.size)
    if (size === undefined) return undefined
    const time = asNumber(o?.time)
    return time === undefined ? {size} : {size, time}
}

const readWitnesses = (value: unknown, servedSize: number | undefined): IWitnessView[] =>
    asArray(value).flatMap((item) => {
        const o = asObject(item)
        const name = asString(o?.name)
        if (!o || !name) return []
        const view: IWitnessView = {name, current: o.current === true, alert: o.alert === true}
        const size = asNumber(o.size)
        if (size !== undefined && servedSize !== undefined && servedSize > size) {
            view.behind = servedSize - size
        }
        const age = asNumber(o.cosigned_age_seconds)
        if (age !== undefined) view.cosignedAgeSeconds = age
        const lastError = asString(o.last_error)
        if (lastError) view.lastError = lastError
        return [view]
    })

const readAnchor = (value: unknown): IHubAnchor | undefined => {
    const time = asNumber(value)
    if (time !== undefined) return {time}
    const o = asObject(value)
    const anchor: IHubAnchor = {}
    const size = asNumber(o?.size)
    if (size !== undefined) anchor.size = size
    const index = asNumber(o?.index)
    if (index !== undefined) anchor.index = index
    const at = asNumber(o?.time)
    if (at !== undefined) anchor.time = at
    return Object.keys(anchor).length > 0 ? anchor : undefined
}

/** The policy's hubs and any others the log reports, with each one's last
 * anchor of the log, when it has one. */
const readHubs = (value: unknown, policyHubs: readonly string[]): IHubView[] => {
    const anchors = asObject(value) ?? {}
    const names = Array.from(new Set(policyHubs.concat(Object.keys(anchors)))).sort()
    return names.map((name) => {
        const anchor = readAnchor(anchors[name])
        return anchor ? {name, anchor} : {name}
    })
}

const readQueue = (value: unknown): IQueueView | undefined => {
    const o = asObject(value)
    const rows = asNumber(o?.rows)
    if (rows === undefined) return undefined
    const oldestSeconds = asNumber(o?.oldest_seconds)
    return oldestSeconds === undefined ? {rows} : {rows, oldestSeconds}
}

const logView = (
    key: string,
    status: TJson | undefined,
    entry: TJson | undefined,
    policyHubs: readonly string[]
): ILogView => {
    const internal = asObject(status?.internal)
    const checkpoint = readCheckpoint(status?.checkpoint)
    const view: ILogView = {
        key,
        frozen: status?.frozen === true || entry?.frozen === true,
        witnesses: readWitnesses(status?.witnesses, checkpoint?.size),
        hubs: readHubs(status?.hubs, policyHubs),
    }
    const origin = asString(status?.origin) ?? asString(entry?.origin)
    if (origin) view.origin = origin
    const visibility = asString(status?.visibility) ?? asString(entry?.visibility)
    if (visibility) view.visibility = visibility
    const size = asNumber(status?.size) ?? asNumber(entry?.size)
    if (size !== undefined) view.size = size
    if (checkpoint) view.checkpoint = checkpoint
    const policy = policyHex(status?.policy) ?? policyHex(entry?.policy)
    if (policy) view.policyHex = policy
    const alert = asString(entry?.alert) ?? asString(internal?.alert)
    if (alert) view.alert = alert
    const queue = readQueue(entry?.queue) ?? readQueue(internal?.queue)
    if (queue) view.queue = queue
    return view
}

/** The event's logs from the sidecar's listing entries and statuses (keyed
 * by the event's name for each log), `ballots` first. */
export const readLogs = (
    logs: unknown,
    status: unknown,
    policyHubs: readonly string[] = []
): ILogView[] => {
    const listing = asArray(logs)
        .map(asObject)
        .filter((o): o is TJson => o !== undefined)
    const statuses = asObject(status) ?? {}
    const matched = new Set<TJson>()
    const views: ILogView[] = []
    for (const key of Object.keys(statuses).sort(byLogOrder)) {
        const s = asObject(statuses[key])
        if (!s) continue
        const name = asString(s.name)
        const origin = asString(s.origin)
        const entry = listing.find(
            (e) =>
                (name !== undefined && e.name === name) ||
                (origin !== undefined && e.origin === origin)
        )
        if (entry) matched.add(entry)
        views.push(logView(key, s, entry, policyHubs))
    }
    for (const entry of listing) {
        const name = asString(entry.name)
        if (name && !matched.has(entry)) views.push(logView(name, undefined, entry, policyHubs))
    }
    return views
}

export interface IEnrolment {
    log: string
    note: string
}

/** Each log's witness enrolment note, for the witnesses' operators. */
export const readEnrolments = (value: unknown): IEnrolment[] =>
    asArray(value).flatMap((item) => {
        const o = asObject(item)
        const log = asString(o?.log)
        const note = asString(o?.note)
        return log && note ? [{log, note}] : []
    })

const LOCAL_HOSTS = ["localhost", "127.0.0.1", "[::1]"]

/** An https URL to link to, or plain http on this machine; nothing else. */
export const safeExternalUrl = (value: unknown): string | undefined => {
    const text = asString(value)
    if (!text) return undefined
    let url: URL
    try {
        url = new URL(text)
    } catch {
        return undefined
    }
    if (url.username || url.password) return undefined
    const local = url.protocol === "http:" && LOCAL_HOSTS.includes(url.hostname)
    return url.protocol === "https:" || local ? url.href : undefined
}
