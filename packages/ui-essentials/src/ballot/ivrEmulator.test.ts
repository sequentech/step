// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Fetching the emulator.
 *
 * Which stage failed is not a detail here: a host shows "not deployed here" for a
 * failed fetch and "something is broken" for anything else, so the two must be told
 * apart by something more reliable than a message.
 */

import {forgetIvrEmulator, IvrEmulatorError, loadIvrEmulator} from "./ivrEmulator"

const shim = {
    default: jest.fn(async () => undefined),
    init: jest.fn(),
    IvrEmulatorDriver: class {},
}

const ok = () => Promise.resolve({ok: true, status: 200} as Response)

describe("loading the emulator", () => {
    let quiet: jest.SpyInstance

    beforeEach(() => {
        forgetIvrEmulator()
        shim.default.mockClear()
        shim.init.mockClear()
        quiet = jest.spyOn(console, "error").mockImplementation(() => undefined)
    })

    afterEach(() => {
        quiet.mockRestore()
        delete (globalThis as {fetch?: unknown}).fetch
    })

    it("asks for the shim and the binary beside one base name", async () => {
        const asked: string[] = []
        globalThis.fetch = jest.fn((url: URL) => {
            asked.push(String(url))
            return ok()
        }) as unknown as typeof fetch
        const imported = jest.fn(async () => shim)

        await loadIvrEmulator("/wasm/ivr_emulator_wasm", imported)

        expect(imported).toHaveBeenCalledWith("http://localhost/wasm/ivr_emulator_wasm.js")
        expect(asked).toEqual(["http://localhost/wasm/ivr_emulator_wasm_bg.wasm"])
    })

    it("drops a query string rather than fetching two names that are not there", async () => {
        globalThis.fetch = jest.fn(ok) as unknown as typeof fetch
        const imported = jest.fn(async () => shim)

        await loadIvrEmulator("/wasm/emu?v=3#top", imported)

        expect(imported).toHaveBeenCalledWith("http://localhost/wasm/emu.js")
    })

    it("initialises the module with the binary it fetched", async () => {
        const response = {ok: true, status: 200} as Response
        globalThis.fetch = jest.fn(async () => response) as unknown as typeof fetch

        await loadIvrEmulator("/wasm/emu", async () => shim)

        expect(shim.default).toHaveBeenCalledWith({module_or_path: response})
        expect(shim.init).toHaveBeenCalled()
    })

    it("calls a missing binary a fetch failure, which is what 'not deployed' looks like", async () => {
        globalThis.fetch = jest.fn(
            async () => ({ok: false, status: 404}) as Response
        ) as unknown as typeof fetch

        await expect(loadIvrEmulator("/wasm/emu", async () => shim)).rejects.toMatchObject({
            operation: "fetch",
        })
    })

    it("calls a broken init an init failure, which is not the same thing", async () => {
        globalThis.fetch = jest.fn(ok) as unknown as typeof fetch
        shim.init.mockImplementationOnce(() => {
            throw new Error("no")
        })

        const failure = await loadIvrEmulator("/wasm/emu", async () => shim).catch((e) => e)

        expect(failure).toBeInstanceOf(IvrEmulatorError)
        expect(failure.operation).toBe("init")
    })

    it("loads once however many times it is asked", async () => {
        globalThis.fetch = jest.fn(ok) as unknown as typeof fetch
        const imported = jest.fn(async () => shim)

        await Promise.all([
            loadIvrEmulator("/wasm/emu", imported),
            loadIvrEmulator("/wasm/emu", imported),
        ])

        expect(imported).toHaveBeenCalledTimes(1)
    })

    it("lets a failed load be tried again", async () => {
        // Opening the panel after a deploy should retry, not repeat the first
        // answer for as long as the tab stays open.
        globalThis.fetch = jest
            .fn()
            .mockImplementationOnce(async () => ({ok: false, status: 502}) as Response)
            .mockImplementation(ok) as unknown as typeof fetch

        await expect(loadIvrEmulator("/wasm/emu", async () => shim)).rejects.toBeInstanceOf(
            IvrEmulatorError
        )
        await expect(loadIvrEmulator("/wasm/emu", async () => shim)).resolves.toBe(shim)
    })
})
