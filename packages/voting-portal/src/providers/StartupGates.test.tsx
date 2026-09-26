// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, cleanup} from "@testing-library/react"
import {WasmGate} from "./WasmWrapper"
import {SettingsWrapper} from "./SettingsContextProvider"

let mockWasmStatus = "loading"
jest.mock("@sequentech/ui-core", () => ({
    useWasm: () => ({status: mockWasmStatus}),
    WasmStatus: {LOADING: "loading", READY: "ready", ERROR: "error"},
}))
// This unit boundary must also resolve after a clean install, before the shared
// package's dist entry exists. The actual startup gates remain under test.
jest.mock("@sequentech/ui-essentials", () => ({Loader: () => <div role="status">Loading</div>}), {
    virtual: true,
})
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const originalFetch = global.fetch
afterEach(() => {
    cleanup()
    global.fetch = originalFetch
})

test("WASM failure replaces the loader with recovery and never opens voting", () => {
    mockWasmStatus = "loading"
    const {rerender} = render(
        <WasmGate>
            <button>Vote</button>
        </WasmGate>
    )
    expect(screen.getByRole("status")).toBeVisible()
    expect(screen.queryByRole("button", {name: "Vote"})).not.toBeInTheDocument()
    mockWasmStatus = "error"
    rerender(
        <WasmGate>
            <button>Vote</button>
        </WasmGate>
    )
    expect(screen.getByRole("alert")).toHaveTextContent("startup.error")
    expect(screen.getByRole("button", {name: "startup.retry"})).toBeVisible()
    expect(screen.queryByRole("status")).not.toBeInTheDocument()
    expect(screen.queryByRole("button", {name: "Vote"})).not.toBeInTheDocument()
    mockWasmStatus = "ready"
    rerender(
        <WasmGate>
            <button>Vote</button>
        </WasmGate>
    )
    expect(screen.getByRole("button", {name: "Vote"})).toBeVisible()
    expect(screen.queryByRole("alert")).not.toBeInTheDocument()
})

test("successful settings open the portal", async () => {
    global.fetch = jest
        .fn()
        .mockResolvedValue({ok: true, json: async () => ({DISABLE_AUTH: false})})
    render(
        <SettingsWrapper>
            <button>Vote</button>
        </SettingsWrapper>
    )
    expect(await screen.findByRole("button", {name: "Vote"})).toBeVisible()
    expect(global.fetch).toHaveBeenCalledWith("/global-settings.json")
    expect(screen.queryByRole("alert")).not.toBeInTheDocument()
})

test.each(["http", "network", "json"])(
    "%s settings failure is recoverable and does not open voting",
    async (fault) => {
        global.fetch = jest.fn().mockImplementation(async () => {
            if (fault === "network") throw new Error("Connection refused")
            return {
                ok: fault !== "http",
                json: async () => {
                    if (fault === "json") throw new SyntaxError("Malformed settings")
                    return {DISABLE_AUTH: true}
                },
            }
        })
        render(
            <SettingsWrapper>
                <button>Vote</button>
            </SettingsWrapper>
        )
        expect(await screen.findByRole("alert")).toHaveTextContent("startup.error")
        expect(screen.getByRole("button", {name: "startup.retry"})).toBeVisible()
        expect(screen.queryByRole("status")).not.toBeInTheDocument()
        expect(screen.queryByRole("button", {name: "Vote"})).not.toBeInTheDocument()
    }
)
