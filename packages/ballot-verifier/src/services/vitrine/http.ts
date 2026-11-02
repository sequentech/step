// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The receipt screen's requests: a trust policy, fetched with GET only, no
// credentials, no referrer and no redirects, at most 64 KiB, from https or
// the local machine; and the requests @vitrine/verify makes, which can be
// held to the policy's own logs.

/** No trust policy is larger. */
export const MAX_POLICY_BYTES = 64 << 10

const DEV_HOSTS = ["localhost", "127.0.0.1", "[::1]"]

export type THttpInit = NonNullable<Parameters<typeof fetch>[1]>

export type FetchLike = (input: string, init?: THttpInit) => Promise<Response>

type TSdkFetch = typeof fetch

/** URLs the verifier may fetch: https, or http to the local machine for
 * development, without credentials in them. */
export const isFetchableUrl = (url: string): boolean => {
    let u: URL
    try {
        u = new URL(url)
    } catch {
        return false
    }
    if (u.username !== "" || u.password !== "") return false
    if (u.protocol === "https:") return u.hostname !== ""
    return u.protocol === "http:" && DEV_HOSTS.includes(u.hostname)
}

/** True when `url` is `base` or a path below it, on the same origin. */
export const isUnder = (url: string, base: string): boolean => {
    try {
        const u = new URL(url)
        const b = new URL(base)
        const dir = b.pathname.endsWith("/") ? b.pathname : `${b.pathname}/`
        return u.origin === b.origin && (u.pathname === b.pathname || u.pathname.startsWith(dir))
    } catch {
        return false
    }
}

const globalFetch: FetchLike = (input, init) => globalThis.fetch(input, init)

/** No JSON answer the verifier reads is larger: an event's public config, or
 * an audited ballot with the ballot style it carries. */
export const MAX_JSON_BYTES = 4 << 20

/** A request that got no usable answer: none, a status other than 200, or
 * one that isn't JSON. */
export class JsonFetchError extends Error {
    override name = "JsonFetchError"
    constructor(
        message: string,
        /** The HTTP status, or 0 when there was no answer. */
        readonly status: number
    ) {
        super(message)
    }
}

export enum EPolicyFetchProblem {
    NOT_HTTPS = "not-https",
    UNREACHABLE = "unreachable",
    STATUS = "status",
    TOO_LARGE = "too-large",
    NOT_TEXT = "not-text",
}

export type TPolicyFetch =
    | {ok: true; text: string}
    | {ok: false; problem: EPolicyFetchProblem; status?: number}

const readLimited = async (response: Response, max: number): Promise<Uint8Array | undefined> => {
    const reader = response.body?.getReader()
    if (!reader) {
        const bytes = new Uint8Array(await response.arrayBuffer())
        return bytes.length > max ? undefined : bytes
    }
    const parts: Uint8Array[] = []
    let size = 0
    for (;;) {
        const {done, value} = await reader.read()
        if (done) break
        size += value.length
        if (size > max) {
            await reader.cancel().catch(() => undefined)
            return undefined
        }
        parts.push(value)
    }
    const bytes = new Uint8Array(size)
    let at = 0
    for (const part of parts) {
        bytes.set(part, at)
        at += part.length
    }
    return bytes
}

/** Fetches JSON with no credentials, no referrer and no redirects, from https
 * or the local machine. Throws a JsonFetchError when it gets none. */
export const fetchJson = async (
    url: string,
    init: THttpInit = {},
    opts: {fetch?: FetchLike; signal?: AbortSignal} = {}
): Promise<unknown> => {
    if (!isFetchableUrl(url)) throw new JsonFetchError("not an https URL", 0)
    const f = opts.fetch ?? globalFetch
    let response: Response
    try {
        response = await f(url, {
            method: "GET",
            ...init,
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
            ...(opts.signal ? {signal: opts.signal} : {}),
        })
    } catch (e) {
        if (opts.signal?.aborted) throw e
        throw new JsonFetchError("no answer", 0)
    }
    if (response.status !== 200) {
        throw new JsonFetchError(`status ${response.status}`, response.status)
    }
    let bytes: Uint8Array | undefined
    try {
        bytes = await readLimited(response, MAX_JSON_BYTES)
    } catch {
        throw new JsonFetchError("no answer", 0)
    }
    if (!bytes) throw new JsonFetchError("answer too large", 413)
    try {
        return JSON.parse(new TextDecoder("utf-8", {fatal: true}).decode(bytes)) as unknown
    } catch {
        throw new JsonFetchError("not JSON", response.status)
    }
}

/** Fetches a trust policy's text. */
export const fetchPolicyText = async (
    url: string,
    opts: {fetch?: FetchLike; signal?: AbortSignal} = {}
): Promise<TPolicyFetch> => {
    if (!isFetchableUrl(url)) return {ok: false, problem: EPolicyFetchProblem.NOT_HTTPS}
    const f = opts.fetch ?? globalFetch
    let response: Response
    try {
        response = await f(url, {
            method: "GET",
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
            ...(opts.signal ? {signal: opts.signal} : {}),
        })
    } catch {
        return {ok: false, problem: EPolicyFetchProblem.UNREACHABLE}
    }
    if (response.status !== 200) {
        return {ok: false, problem: EPolicyFetchProblem.STATUS, status: response.status}
    }
    let bytes: Uint8Array | undefined
    try {
        bytes = await readLimited(response, MAX_POLICY_BYTES)
    } catch {
        return {ok: false, problem: EPolicyFetchProblem.UNREACHABLE}
    }
    if (!bytes) return {ok: false, problem: EPolicyFetchProblem.TOO_LARGE}
    try {
        return {ok: true, text: new TextDecoder("utf-8", {fatal: true}).decode(bytes)}
    } catch {
        return {ok: false, problem: EPolicyFetchProblem.NOT_TEXT}
    }
}

const urlOf = (input: Parameters<TSdkFetch>[0]): string =>
    typeof input === "string" ? input : input instanceof URL ? input.href : input.url

/** The fetch @vitrine/verify gets: it reaches https URLs (or the local
 * machine) only, and only those under `bases` when they are given. Anything
 * else fails as a network error would, which the SDK reports as unreachable. */
export const sdkFetch = (f: FetchLike = globalFetch, bases?: readonly string[]): TSdkFetch =>
    ((input: Parameters<TSdkFetch>[0], init?: THttpInit) => {
        const url = urlOf(input)
        if (!isFetchableUrl(url) || (bases && !bases.some((base) => isUnder(url, base)))) {
            return Promise.reject(new TypeError("This request is not allowed in this round."))
        }
        return f(url, init)
    }) as TSdkFetch
