// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {defineBrowserCommand} from "@vitest/browser-playwright"
import type {Page, Route} from "playwright"

interface RunnerSockets {
    origin: string
    hmrToken: string
    apiToken: string
    sessionId: string
    projectName: string
}

interface SocketState {
    runner: RunnerSockets
    pending: Promise<void>[]
}

// This function is serialized into each browser document. Playwright's socket
// routing only starts on the next navigation, after Storybook's setup hook.
const installSocketGuard = (runner: RunnerSockets) => {
    const key = Symbol.for("sequent.storybook.websockets")
    const target = globalThis as typeof globalThis & {
        [key: symbol]: SocketState | undefined
        __sequentReportWebSocket: (message: string) => Promise<void>
    }
    const existing = target[key]
    if (existing) {
        existing.runner = runner
        return
    }
    const state: SocketState = {runner, pending: []}
    target[key] = state
    const isRunnerSocket = (url: URL, protocols: string[], runner: RunnerSockets) => {
        const origin = new URL(runner.origin)
        if (
            url.host !== origin.host ||
            url.protocol !== (origin.protocol === "https:" ? "wss:" : "ws:") ||
            url.username ||
            url.password ||
            url.hash
        )
            return false
        const query = url.searchParams
        if (url.pathname === "/")
            return (
                protocols.length === 1 &&
                protocols[0] === "vite-hmr" &&
                query.size === 1 &&
                Boolean(runner.hmrToken) &&
                query.get("token") === runner.hmrToken
            )
        const keys = ["type", "rpcId", "sessionId", "projectName", "method", "token"]
        return (
            url.pathname === "/__vitest_browser_api__" &&
            protocols.length === 0 &&
            query.size === keys.length &&
            keys.every((key) => query.getAll(key).length === 1) &&
            ["tester", "orchestrator"].includes(query.get("type") ?? "") &&
            Boolean(query.get("rpcId")) &&
            (query.get("type") !== "orchestrator" || query.get("rpcId") === runner.sessionId) &&
            query.get("sessionId") === runner.sessionId &&
            query.get("projectName") === runner.projectName &&
            ["run", "collect", "none"].includes(query.get("method") ?? "") &&
            Boolean(runner.apiToken) &&
            query.get("token") === runner.apiToken
        )
    }
    globalThis.WebSocket = new Proxy(WebSocket, {
        construct: (nativeSocket, args, newTarget) => {
            const [address, requested] = args as ConstructorParameters<typeof WebSocket>
            const url = new URL(String(address), location.href)
            // WebSocket accepts HTTP(S) URLs and converts their scheme.
            if (url.protocol === "http:") url.protocol = "ws:"
            if (url.protocol === "https:") url.protocol = "wss:"
            const protocols = typeof requested === "string" ? [requested] : (requested ?? [])
            if (!isRunnerSocket(url, protocols, state.runner)) {
                // Preserve evidence in Node even when the app catches or navigates.
                state.pending.push(
                    target.__sequentReportWebSocket(`Unexpected WebSocket: ${url.href}`)
                )
                throw new DOMException("Unexpected story WebSocket", "SecurityError")
            }
            return Reflect.construct(nativeSocket, args, newTarget)
        },
    })
}

interface Guard {
    unexpected: string[]
    route: (route: Route) => Promise<void>
    active: boolean
}

const guards = new WeakMap<Page, Guard>()

export const installNetworkGuard = async (page: Page, runner: RunnerSockets) => {
    let guard = guards.get(page)
    if (guard?.active) throw new Error("Network guard is already installed")
    if (guard) {
        guard.active = true
        await Promise.all(page.frames().map((frame) => frame.evaluate(installSocketGuard, runner)))
        await page.route("**/*", guard.route)
        return
    }
    const origin = new URL(page.url()).origin
    const unexpected: string[] = []
    const route = async (request: Route) => {
        const url = new URL(request.request().url())
        // Vite modules, local static assets and WASM are the only real requests.
        // Application responses must be provided by a story's boundary fixture.
        const localAsset =
            ["GET", "HEAD"].includes(request.request().method()) &&
            url.origin === origin &&
            (url.pathname.startsWith("/@") ||
                url.pathname.includes("/node_modules/") ||
                /\.(?:[cm]?[jt]sx?|css|wasm|png|jpe?g|svg|woff2?|ttf|ico|map)$/.test(url.pathname))
        if (localAsset) return request.continue()
        unexpected.push(`${request.request().method()} ${url.href}`)
        await request.abort("blockedbyclient")
    }
    guard = {unexpected, route, active: true}
    guards.set(page, guard)
    await page.exposeFunction("__sequentReportWebSocket", (message: string) => {
        unexpected.push(message)
    })
    // Install once per page for future frames, and immediately in loaded frames.
    // The browser-side symbol prevents wrapping the native constructor twice.
    await page.addInitScript(installSocketGuard, runner)
    await Promise.all(page.frames().map((frame) => frame.evaluate(installSocketGuard, runner)))
    await page.route("**/*", route)
}

export const collectNetworkViolations = async (page: Page) => {
    const guard = guards.get(page)
    if (!guard?.active) throw new Error("Network guard was not installed")
    await page.unroute("**/*", guard.route)
    guard.active = false
    await Promise.all(
        page.frames().map((frame) =>
            frame.evaluate(async () => {
                const target = globalThis as typeof globalThis & {
                    [key: symbol]: SocketState | undefined
                }
                await Promise.all(
                    target[Symbol.for("sequent.storybook.websockets")]?.pending.splice(0) ?? []
                )
            })
        )
    )
    return guard.unexpected.splice(0)
}

export const startNetworkGuard = defineBrowserCommand(async ({page, project, sessionId}) =>
    installNetworkGuard(page, {
        origin: new URL(page.url()).origin,
        hmrToken: project.browser?.vite.config.webSocketToken ?? "",
        apiToken: project.vitest.config.api.token,
        sessionId,
        projectName: project.name,
    })
)

export const finishNetworkGuard = defineBrowserCommand(async ({page}) =>
    collectNetworkViolations(page)
)
