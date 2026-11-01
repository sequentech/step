// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Problems the hubs of an election's policy have recorded (DECISIONS D4 and
// D8). A hub lists them at `GET <hub>/problems?policy=sha256:<hex>` as
// `{hub, problems: [{id, kind, subject, policy, logged_at, entry}], more}`.
// Each is a pointer to an evidence file that anyone can check on the hub's
// site, never the proof itself.
//
// The Transparency tab's banner built from these lists is a convenience: it
// works only while this portal tells the truth, and says so. This module
// holds every call to the problems API, so that a change to the server's
// shapes touches one place; the SDK has no client for it yet. Availability
// reports (a hub or log that stopped answering) are not hub evidence and are
// never read here.

import type {Policy} from "@vitrine/verify"
import {safeExternalUrl} from "@/services/Transparency"

/** Rows asked for at once (the hub's maximum is 500) and pages followed. */
export const PROBLEM_ROWS = 50
export const MAX_PROBLEM_PAGES = 2
/** A list of 500 rows is far smaller. */
const MAX_BODY_BYTES = 1 << 20
const TIMEOUT_MS = 10_000

const EVIDENCE_ID = /^sha256:[0-9a-f]{64}$/
const KIND = /^[a-z][a-z0-9-]{0,63}$/

/** A problem as a hub lists it. */
export interface IHubProblem {
    /** The evidence ID, `sha256:` and 64 hex digits. */
    id: string
    /** `split-view`, `broken-promise`, ... */
    kind: string
    /** When the hub logged it, POSIX seconds. */
    loggedAt: number
    /** The hub's log entry that records it. */
    entry: number
}

export interface IHubProblemPage {
    problems: IHubProblem[]
    /** Older problems remain. */
    more: boolean
}

/** Reads an answer of `GET problems`; rows without an evidence ID, a kind, a
 * time and an entry index, or for another policy, are left out, so that a hub
 * cannot make the banner show a blank row. A TypeError for what is not a
 * list. */
export const parseProblemPage = (text: string, policyId: string): IHubProblemPage => {
    let value: unknown
    try {
        value = JSON.parse(text)
    } catch {
        throw new TypeError("not a list of problems")
    }
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new TypeError("not a list of problems")
    }
    const object = value as Record<string, unknown>
    if (!Array.isArray(object.problems)) throw new TypeError("not a list of problems")
    const problems: IHubProblem[] = []
    for (const raw of object.problems.slice(0, 500)) {
        if (typeof raw !== "object" || raw === null || Array.isArray(raw)) continue
        const row = raw as Record<string, unknown>
        const {id, kind, policy} = row
        const at = row.logged_at
        const entry = row.entry
        if (typeof id !== "string" || !EVIDENCE_ID.test(id)) continue
        if (typeof kind !== "string" || !KIND.test(kind)) continue
        if (policy !== policyId) continue
        if (typeof at !== "number" || !Number.isSafeInteger(at) || at < 0) continue
        if (typeof entry !== "number" || !Number.isSafeInteger(entry) || entry < 0) continue
        problems.push({id, kind, loggedAt: at, entry})
    }
    return {problems, more: object.more === true}
}

/** The URL of a hub's list for a policy. */
export const problemsUrl = (hubUrl: string, policyId: string, before?: number): string => {
    const base = hubUrl.endsWith("/") ? hubUrl : `${hubUrl}/`
    return `${base}problems?policy=${encodeURIComponent(policyId)}&limit=${PROBLEM_ROWS}${
        before === undefined ? "" : `&before=${before}`
    }`
}

export type TFetch = (input: string, init?: Parameters<typeof fetch>[1]) => Promise<Response>

export interface IHubProblemsOptions {
    fetch?: TFetch
    signal?: AbortSignal
}

const getText = async (url: string, opts: IHubProblemsOptions): Promise<string> => {
    if (!safeExternalUrl(url)) throw new Error("not a hub address")
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), TIMEOUT_MS)
    const onAbort = () => controller.abort()
    opts.signal?.addEventListener("abort", onAbort)
    try {
        const send: TFetch = opts.fetch ?? ((input, init) => globalThis.fetch(input, init))
        const response = await send(url, {
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
            signal: controller.signal,
        })
        if (response.status !== 200) throw new Error(`the hub answered ${response.status}`)
        const length = Number(response.headers?.get("content-length"))
        if (Number.isFinite(length) && length > MAX_BODY_BYTES) throw new Error("too large")
        const text = await response.text()
        if (text.length > MAX_BODY_BYTES) throw new Error("too large")
        return text
    } finally {
        clearTimeout(timer)
        opts.signal?.removeEventListener("abort", onAbort)
    }
}

/** The problems one hub lists for the policy, newest first. Throws when the
 * hub does not answer, or answers with something else. */
export const fetchHubProblems = async (
    hubUrl: string,
    policyId: string,
    opts: IHubProblemsOptions = {}
): Promise<IHubProblem[]> => {
    const all: IHubProblem[] = []
    let before: number | undefined
    for (let page = 0; page < MAX_PROBLEM_PAGES; page++) {
        const got = parseProblemPage(
            await getText(problemsUrl(hubUrl, policyId, before), opts),
            policyId
        )
        all.push(...got.problems)
        const last = got.problems[got.problems.length - 1]
        // Paging goes by the entry index; a page that does not move on ends it.
        if (!got.more || !last || (before !== undefined && last.entry >= before)) break
        before = last.entry
    }
    return all
}

/** A problem with the hubs that list it. */
export interface IListedProblem extends IHubProblem {
    /** The names of the hubs that list it, as the policy names them. */
    hubs: string[]
}

export interface IHubProblems {
    /** Each problem once, the newest first. */
    problems: IListedProblem[]
    /** The hubs that did not answer. Silence is never shown as a problem. */
    unreachable: string[]
}

/** Asks every hub of the policy which problems it lists. A hub that does not
 * answer is left out; nothing here throws. */
export const hubProblems = async (
    policy: Policy,
    opts: IHubProblemsOptions = {}
): Promise<IHubProblems> => {
    const hubs = Array.from(policy.hubs.values())
    const answers = await Promise.all(
        hubs.map(async (hub) => {
            try {
                return {hub: hub.name, problems: await fetchHubProblems(hub.url, policy.id, opts)}
            } catch {
                return {hub: hub.name, problems: undefined}
            }
        })
    )
    const byId = new Map<string, IListedProblem>()
    for (const answer of answers) {
        for (const problem of answer.problems ?? []) {
            const seen = byId.get(problem.id)
            if (!seen) byId.set(problem.id, {...problem, hubs: [answer.hub]})
            else if (!seen.hubs.includes(answer.hub)) seen.hubs.push(answer.hub)
        }
    }
    return {
        problems: Array.from(byId.values()).sort((a, b) => b.loggedAt - a.loggedAt),
        unreachable: answers.filter((answer) => answer.problems === undefined).map((a) => a.hub),
    }
}

/** The translation key for a problem kind, in the plain words of D4. A kind
 * this portal does not know is worded generally, never as its code. */
export const problemKindKey = (kind: string): string => {
    switch (kind) {
        case "split-view":
            return "transparencyTab.hubProblems.kind.splitView"
        case "broken-promise":
            return "transparencyTab.hubProblems.kind.brokenPromise"
        case "bad-entry":
        case "hub-entry":
        case "time-regression":
        case "bad-checkpoint":
            return "transparencyTab.hubProblems.kind.breaksRules"
        case "policy-equivocation":
            return "transparencyTab.hubProblems.kind.twoPolicies"
        case "unlogged-code":
            return "transparencyTab.hubProblems.kind.unloggedCode"
        default:
            return "transparencyTab.hubProblems.kind.other"
    }
}

/** The page of the problem on a hub's site, where the evidence is checked in
 * the browser: `<site>#policy=<id>&problem=<hex>`. Undefined when the site is
 * not an address to link to. */
export const problemLink = (
    site: string | undefined,
    policyId: string,
    evidenceId: string
): string | undefined => {
    const safe = safeExternalUrl(site)
    if (!safe) return undefined
    const params = new URLSearchParams({
        policy: policyId,
        problem: evidenceId.slice("sha256:".length),
    })
    const hash = safe.indexOf("#")
    return `${hash < 0 ? safe : safe.slice(0, hash)}#${params.toString()}`
}
