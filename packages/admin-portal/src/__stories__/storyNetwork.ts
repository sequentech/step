// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The admin preview installs this guard around every story, in the Storybook
// UI as in the test runner: requests leave the page only for the Storybook
// server's own modules and assets, and a story answers any other request
// explicitly. Boundaries register here so that one check covers all of them.
import {routeFetch, type FetchService} from "@sequentech/ui-test-kit/adapters/fetch"
import type {MockRequest, MockResponse} from "@sequentech/ui-test-kit/mocks/http"
import {ViolationLog} from "@sequentech/ui-test-kit/mocks/violations"

/** A service boundary of the story; its `unexpected` list must stay empty. */
export interface StoryBoundary {
    unexpected: string[]
}

interface ActiveStory {
    services: FetchService[]
    violations: ViolationLog
    boundaries: StoryBoundary[]
    opened: string[]
}

let active: ActiveStory | undefined

/** Registers a boundary for the end-of-story check; boundaries outside a story are ignored. */
export function registerBoundary<T extends StoryBoundary>(boundary: T): T {
    active?.boundaries.push(boundary)
    return boundary
}

const isLocal = (address: string | URL) => {
    const url = new URL(String(address), globalThis.location.href)
    return url.origin === globalThis.location.origin || ["blob:", "data:"].includes(url.protocol)
}

/** Installs the guard; returns the function that removes it. */
export function guardStoryNetwork(): () => void {
    const story: ActiveStory = {
        services: [],
        violations: new ViolationLog(),
        boundaries: [],
        opened: [],
    }
    active = story
    const restoreFetch = routeFetch(story.services, story.violations)
    const blocked = (kind: string, address: string | URL) => {
        const message = `Unexpected ${kind}: ${String(address)}`
        story.violations.add(message)
        console.error(message)
    }
    const {open: xhrOpen} = XMLHttpRequest.prototype
    XMLHttpRequest.prototype.open = function (
        this: XMLHttpRequest,
        method: string,
        url: string | URL,
        ...rest: unknown[]
    ) {
        if (!isLocal(url)) {
            blocked(`XMLHttpRequest ${method}`, url)
            throw new DOMException("Unexpected story request", "SecurityError")
        }
        return Reflect.apply(xhrOpen, this, [method, url, ...rest])
    } as typeof xhrOpen
    const NativeEventSource = globalThis.EventSource
    globalThis.EventSource = new Proxy(NativeEventSource, {
        construct(target, args: ConstructorParameters<typeof EventSource>, newTarget) {
            if (!isLocal(args[0])) {
                blocked("EventSource", args[0])
                throw new DOMException("Unexpected story event stream", "SecurityError")
            }
            return Reflect.construct(target, args, newTarget)
        },
    })
    const NativeWebSocket = globalThis.WebSocket
    globalThis.WebSocket = new Proxy(NativeWebSocket, {
        construct(target, args: ConstructorParameters<typeof WebSocket>, newTarget) {
            // Storybook and the test runner keep their own sockets to this host.
            const url = new URL(String(args[0]), globalThis.location.href)
            if (url.host !== globalThis.location.host) {
                blocked("WebSocket", url)
                throw new DOMException("Unexpected story WebSocket", "SecurityError")
            }
            return Reflect.construct(target, args, newTarget)
        },
    })
    const {sendBeacon} = navigator
    navigator.sendBeacon = (url, data) => {
        if (!isLocal(url)) {
            blocked("beacon", url)
            return false
        }
        return sendBeacon.call(navigator, url, data)
    }
    const {open: windowOpen} = window
    // A new window would load its address; the story records it instead.
    window.open = (url?: string | URL) => {
        story.opened.push(String(url ?? ""))
        return null
    }
    return () => {
        restoreFetch()
        XMLHttpRequest.prototype.open = xhrOpen
        globalThis.EventSource = NativeEventSource
        globalThis.WebSocket = NativeWebSocket
        navigator.sendBeacon = sendBeacon
        window.open = windowOpen
        if (active === story) active = undefined
    }
}

/** Unexpected requests and boundary operations of the current story. */
export function storyViolations(): string[] {
    if (!active) return []
    return [
        ...active.violations.list(),
        ...active.boundaries.flatMap((boundary) => boundary.unexpected),
    ]
}

/** Addresses the story asked `window.open` to load. */
export const openedWindows = (): string[] => [...(active?.opened ?? [])]

export interface FetchCall {
    method: string
    url: string
    headers: Record<string, string>
    body?: string
}

/**
 * Answers `fetch` requests whose address starts with one of the keys, e.g.
 * presigned upload URLs; the calls are recorded in order. Other requests
 * remain violations.
 */
export function storyFetch(
    routes: Record<string, (request: MockRequest) => MockResponse | Promise<MockResponse>>
) {
    const calls: FetchCall[] = []
    const route = (url: URL) =>
        Object.keys(routes)
            .filter((prefix) => url.href.startsWith(prefix))
            .sort((a, b) => b.length - a.length)[0]
    const service: FetchService = {
        handles: (url) => route(url) !== undefined,
        handle: (request) => {
            calls.push({
                method: request.method,
                url: request.url.href,
                headers: request.headers,
                body: request.body,
            })
            return routes[route(request.url) as string](request)
        },
    }
    active?.services.push(service)
    return {calls}
}
