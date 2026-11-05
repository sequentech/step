// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Requests to Vitrine's logs and monitors: no credentials, no referrer, no
// redirects, bounded bodies. Network errors read as status 0, so that nothing
// here can throw into a cast.

/** A whole receipt chain is at most 4 MiB, and nothing Vitrine serves is larger. */
export const MAX_RESPONSE_BYTES = 4 << 20

const DEV_HOSTS = ["localhost", "127.0.0.1", "[::1]"]

export type THttpInit = NonNullable<Parameters<typeof fetch>[1]>

export type FetchLike = (input: string, init?: THttpInit) => Promise<Response>

export interface IHttpOptions {
    fetch?: FetchLike
    signal?: AbortSignal
}

export interface IHttpResponse {
    status: number
    body?: string
}

/** `base` and `path` with exactly one slash between them. */
export const joinUrl = (base: string, path: string): string =>
    base.endsWith("/") ? `${base}${path}` : `${base}/${path}`

/** URLs the portal may fetch: https, or http to the local machine for
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

const request = async (
    url: string,
    init: THttpInit,
    opts: IHttpOptions
): Promise<IHttpResponse> => {
    if (!isFetchableUrl(url)) return {status: 0}
    const f: FetchLike = opts.fetch ?? ((input, i) => globalThis.fetch(input, i))
    let response: Response
    try {
        response = await f(url, {
            ...init,
            credentials: "omit",
            referrerPolicy: "no-referrer",
            redirect: "error",
            ...(opts.signal ? {signal: opts.signal} : {}),
        })
    } catch {
        return {status: 0}
    }
    try {
        const length = Number(response.headers?.get("content-length"))
        if (Number.isFinite(length) && length > MAX_RESPONSE_BYTES) {
            await response.body?.cancel().catch(() => undefined)
            return {status: 413}
        }
        const reader = response.body?.getReader()
        const parts: Uint8Array[] = []
        let size = 0
        if (reader) {
            for (;;) {
                const next = await reader.read()
                if (next.done) break
                size += next.value.length
                if (size > MAX_RESPONSE_BYTES) {
                    await reader.cancel().catch(() => undefined)
                    return {status: 413}
                }
                parts.push(next.value)
            }
        } else {
            const bytes = new Uint8Array(await response.arrayBuffer())
            if (bytes.length > MAX_RESPONSE_BYTES) return {status: 413}
            parts.push(bytes)
            size = bytes.length
        }
        const bytes = new Uint8Array(size)
        let at = 0
        for (const part of parts) {
            bytes.set(part, at)
            at += part.length
        }
        return {
            status: response.status,
            body: new TextDecoder("utf-8", {fatal: true}).decode(bytes),
        }
    } catch {
        return {status: 0}
    }
}

export const httpGet = (url: string, opts: IHttpOptions = {}): Promise<IHttpResponse> =>
    request(url, {method: "GET"}, opts)

export const httpPost = (
    url: string,
    body: string,
    contentType: string,
    opts: IHttpOptions = {}
): Promise<IHttpResponse> =>
    request(url, {method: "POST", body, headers: {"Content-Type": contentType}}, opts)
