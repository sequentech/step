/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {expect, it, jest, afterEach} from "@jest/globals"
import {act, cleanup, render, renderHook} from "@testing-library/react"
import {initCore} from "./wasm"
import {WasmContextProvider, useWasm} from "./WasmContext"

jest.mock("./wasm", () => ({initCore: jest.fn()}))
afterEach(() => {
    cleanup()
    jest.resetAllMocks()
})

function Status() {
    return <output aria-label="Ballot engine">{useWasm().status}</output>
}

it.each(["ready", "error"])(
    "keeps consumers loading until initialization becomes %s",
    async (outcome) => {
        let resolve!: () => void
        let reject!: (error: Error) => void
        const initialization = new Promise<void>((done, fail) => {
            resolve = done
            reject = fail
        })
        jest.mocked(initCore).mockReturnValue(initialization)
        const view = render(
            <WasmContextProvider>
                <Status />
            </WasmContextProvider>
        )
        expect(view.getByLabelText("Ballot engine").textContent).toBe("loading")
        await act(async () => {
            if (outcome === "ready") resolve()
            else reject(new Error("synthetic WASM load failure"))
            await initialization.catch(() => undefined)
        })
        expect(view.getByLabelText("Ballot engine").textContent).toBe(outcome)
        expect(initCore).toHaveBeenCalledTimes(1)
    }
)

it("explains a missing provider instead of silently pretending the engine is ready", () => {
    expect(() => renderHook(useWasm)).toThrow("useWasm must be used within a WasmProvider")
})
