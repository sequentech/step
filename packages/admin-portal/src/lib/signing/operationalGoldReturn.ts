// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {INITIALIZE_RETURN_PATH, localInitializeBuild} from "./localInitializeAuthority"

/**
 * Only unprivileged opaque IDs sit in tab-local resume storage. The
 * normal Admin JWT and the single-use Gold authorization response never
 * persist here, travel in a URL created by the app, or authorize status.
 */
const MARKER = "sequent.operational-signing.tab.v1"
const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/
export const isId = (s: unknown): s is string =>
    typeof s === "string" && UUID.test(s) && s !== "00000000-0000-0000-0000-000000000000"
const object = (v: unknown): Record<string, unknown> =>
    v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : {}

export type EventPageKind = "event-base" | "event-show" | "event-edit"
export type ElectionPageKind = "election-base" | "election-show" | "election-edit"
export type SafePanel = "publish" | "signatures-requests"
export type IReturnPage =
    | {path: EventPageKind; panel: SafePanel; election_id?: undefined}
    | {path: ElectionPageKind; panel: "publish"; election_id: string}
export interface ITabMarker {
    tenant: string
    event: string
    request: string
    official: string
    attempt: string
    page: IReturnPage
}
export type ReturnSecret = {state: string; code: string; issuer?: string}

export function parsePage(value: unknown): IReturnPage | undefined {
    const p = object(value)
    if (Object.keys(p).some((k) => k !== "path" && k !== "panel" && k !== "election_id"))
        return undefined
    if (p.path === "event-base" || p.path === "event-show" || p.path === "event-edit") {
        if ((p.panel !== "publish" && p.panel !== "signatures-requests") || p.election_id != null)
            return undefined
        return {path: p.path, panel: p.panel}
    }
    if (p.path === "election-base" || p.path === "election-show" || p.path === "election-edit") {
        if (p.panel !== "publish" || !isId(p.election_id)) return undefined
        return {path: p.path, panel: p.panel, election_id: p.election_id}
    }
    return undefined
}

export function observedPage(
    pathname: string,
    event: string,
    activePanel: SafePanel | undefined,
    election?: string
): IReturnPage | undefined {
    if (!isId(event) || !activePanel) return undefined
    if (election !== undefined) {
        if (!isId(election) || activePanel !== "publish") return undefined
        const base = "/sequent_backend_election/" + election
        const path =
            pathname === base
                ? "election-base"
                : pathname === base + "/show"
                  ? "election-show"
                  : pathname === base + "/edit"
                    ? "election-edit"
                    : undefined
        return path && parsePage({path, panel: activePanel, election_id: election})
    }
    const base = "/sequent_backend_election_event/" + event
    const path =
        pathname === base
            ? "event-base"
            : pathname === base + "/show"
              ? "event-show"
              : pathname === base + "/edit"
                ? "event-edit"
                : undefined
    return path && parsePage({path, panel: activePanel})
}

export function signingReturnPath(event: string, page: IReturnPage): string {
    const normalized = parsePage(page)
    if (!isId(event) || !normalized) throw new Error("Invalid signing return page")
    const base = normalized.election_id
        ? "/sequent_backend_election/" + normalized.election_id
        : "/sequent_backend_election_event/" + event
    const suffix = normalized.path.endsWith("-show")
        ? "/show"
        : normalized.path.endsWith("-edit")
          ? "/edit"
          : ""
    return base + suffix
}
export function decodeTabMarker(raw: unknown): ITabMarker | undefined {
    const m = object(raw)
    const page = parsePage(m.page)
    if (
        Object.keys(m).length !== 6 ||
        !page ||
        !isId(m.tenant) ||
        !isId(m.event) ||
        !isId(m.request) ||
        !isId(m.official) ||
        !isId(m.attempt)
    )
        return undefined
    return {
        tenant: m.tenant,
        event: m.event,
        request: m.request,
        official: m.official,
        attempt: m.attempt,
        page,
    }
}
export function markBeforeFullRedirect(storage: Storage, marker: ITabMarker): boolean {
    if (!decodeTabMarker(marker)) return false
    try {
        storage.setItem(MARKER, JSON.stringify(decodeTabMarker(marker)))
        return true
    } catch {
        return false
    }
}
export function readTabMarker(storage: Storage): ITabMarker | undefined {
    try {
        const raw = storage.getItem(MARKER)
        return raw && raw.length <= 1024 ? decodeTabMarker(JSON.parse(raw)) : undefined
    } catch {
        return undefined
    }
}

export function clearTabMarker(storage: Storage): void {
    try {
        storage.removeItem(MARKER)
    } catch {
        /* nothing confidential retained */
    }
}

/** Run as the first operation of the dedicated return entry point. */
export function captureGoldReturn(
    location: Pick<Location, "pathname" | "search" | "hash" | "protocol"> &
        Partial<Pick<Location, "origin" | "href">>,
    clean: (path: string) => void
): ReturnSecret | undefined {
    if (location.pathname !== INITIALIZE_RETURN_PATH) return undefined
    const fragment = location.hash
    const query = location.search
    const local = localInitializeBuild(location.origin)
    const accepted =
        location.protocol === "https:" ||
        (local && location.protocol === "http:" &&
            location.href === location.origin + INITIALIZE_RETURN_PATH + fragment)
    try {
        clean(INITIALIZE_RETURN_PATH)
    } catch {
        return undefined
    }
    if (!accepted || query || !fragment.startsWith("#") || fragment.length > 8192)
        return undefined
    const parts = new URLSearchParams(fragment.slice(1))
    const keys = Array.from(parts.keys())
    if (
        keys.some((x) => !["state", "code", "session_state", "iss"].includes(x)) ||
        new Set(keys).size !== keys.length
    )
        return undefined
    const s = parts.get("state"),
        code = parts.get("code")
    if (!s || !/^[A-Za-z0-9_-]{43}$/.test(s) || !code || !/^[!-~]{1,4096}$/.test(code))
        return undefined
    if (local) {
        const issuer = parts.get("iss")
        const prefix = "http://localhost:18090/realms/tenant-"
        if (!issuer?.startsWith(prefix) || !isId(issuer.slice(prefix.length))) return undefined
        return {state: s, code, issuer}
    }
    return {state: s, code}
}

export function bindReturnedPage(
    previous: ITabMarker,
    response: unknown,
    currentUser: {tenant: string; official: string}
): IReturnPage | undefined {
    const result = object(response)
    const page = parsePage(result.page)
    return currentUser.tenant === previous.tenant &&
        currentUser.official === previous.official &&
        result.tenant === previous.tenant &&
        result.official === previous.official &&
        result.event === previous.event &&
        result.request === previous.request &&
        result.attempt === previous.attempt &&
        page?.path === previous.page.path &&
        page?.panel === previous.page.panel &&
        page?.election_id === previous.page.election_id
        ? page
        : undefined
}

const DISPLAY = "sequent.operational-signing.display.v1"
export type VisibleResult = "approval-recorded" | "reconcile" | "setup-returned"
export interface IDisplayReturn {
    marker: ITabMarker
    result: VisibleResult
    path: string
}

/** This non-credential hint reopens only the original request drawer; it
 * must never start or finish a ceremony. The real panel refreshes status. */
export function saveDisplayReturn(storage: Storage, value: IDisplayReturn): void {
    const marker = decodeTabMarker(value.marker)
    if (!marker || signingReturnPath(marker.event, marker.page) !== value.path) return
    try {
        storage.setItem(
            DISPLAY,
            JSON.stringify({
                marker,
                result: value.result,
                path: value.path,
                until: Date.now() + 120_000,
            })
        )
    } catch {
        /* normal record can still be found through the request list */
    }
}
export function readDisplayReturn(
    storage: Storage,
    path: string,
    scope?: {tenant: string; official: string}
): IDisplayReturn | undefined {
    try {
        const raw = storage.getItem(DISPLAY)
        if (!raw || raw.length > 1536) return undefined
        const value = object(JSON.parse(raw))
        const marker = decodeTabMarker(value.marker)
        const result = value.result
        if (
            !marker ||
            typeof value.until !== "number" ||
            value.until < Date.now() ||
            value.until > Date.now() + 120000 ||
            value.path !== path ||
            signingReturnPath(marker.event, marker.page) !== path ||
            (result !== "approval-recorded" &&
                result !== "reconcile" &&
                result !== "setup-returned") ||
            (scope && (scope.tenant !== marker.tenant || scope.official !== marker.official))
        )
            return undefined
        return {marker, result, path}
    } catch {
        return undefined
    }
}
export function discardDisplayReturn(storage: Storage): void {
    try {
        storage.removeItem(DISPLAY)
    } catch {
        /* no operational effect */
    }
}
