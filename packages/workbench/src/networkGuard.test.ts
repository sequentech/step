// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, test, vi} from "vitest"
import {emulatorAllowance, installNetworkGuard, type BlockedRequest} from "./networkGuard"

const origin = "http://127.0.0.1:5173"

function guarded(allowed?: (url: URL) => boolean) {
    const passed = vi.fn(async () => new Response("ok"))
    const target = {
        fetch: passed as typeof fetch,
        location: new URL(`${origin}/#/x`) as unknown as Location,
    }
    const blocked: BlockedRequest[] = []
    const restore = installNetworkGuard(target, (request) => blocked.push(request), allowed)
    return {target, passed, blocked, restore}
}

test("same-origin reads reach the server", async () => {
    const {target, passed, blocked} = guarded()
    await target.fetch("/node_modules/sequent-core/index_bg.wasm")
    await target.fetch(new Request(`${origin}/src/main.tsx`))
    expect(passed).toHaveBeenCalledTimes(2)
    expect(blocked).toEqual([])
})

test.each([
    ["https://hasura.example/v1/graphql", {method: "POST"}, "POST"],
    ["https://keycloak.example/realms/x", undefined, "GET"],
    [`${origin}/global-settings.json`, {method: "PUT"}, "PUT"],
])("%s is refused and reported", async (url, init, method) => {
    const {target, passed, blocked} = guarded()
    await expect(target.fetch(url, init)).rejects.toThrow(
        `The workbench blocks ${method} ${url} in offline mode`
    )
    expect(passed).not.toHaveBeenCalled()
    expect(blocked).toEqual([{method, url}])
})

test("a request object's method counts", async () => {
    const {target, blocked} = guarded()
    await expect(target.fetch(new Request(`${origin}/api`, {method: "DELETE"}))).rejects.toThrow()
    expect(blocked).toEqual([{method: "DELETE", url: `${origin}/api`}])
})

test("restoring gives back the original fetch", () => {
    const {target, passed, restore} = guarded()
    restore()
    void target.fetch("https://example.org")
    expect(passed).toHaveBeenCalledWith("https://example.org")
})

test("an emulator a framing tool names is fetched from its origin, and nothing else is", async () => {
    const emulators = emulatorAllowance()
    const {target, passed, blocked} = guarded(emulators.allowed)
    const wasm = "http://architect.example/wasm/ivr_emulator_wasm_bg.wasm"
    await expect(target.fetch(wasm)).rejects.toThrow()

    emulators.allow("http://architect.example/wasm/ivr_emulator_wasm?v=2#x")
    await target.fetch(wasm)
    await target.fetch("http://architect.example/wasm/ivr_emulator_wasm.js")
    expect(passed).toHaveBeenCalledTimes(2)

    for (const other of [
        "http://architect.example/wasm/sequent_core_bg.wasm",
        `${wasm}?x=1`,
        "http://architect.example/api/graphql",
    ])
        await expect(target.fetch(other)).rejects.toThrow()
    await expect(target.fetch(wasm, {method: "POST"})).rejects.toThrow()
    expect(blocked.map(({url}) => url)).toEqual([
        wasm,
        "http://architect.example/wasm/sequent_core_bg.wasm",
        `${wasm}?x=1`,
        "http://architect.example/api/graphql",
        wasm,
    ])
})
