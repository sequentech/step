// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderToStaticMarkup} from "react-dom/server"
import {WasmGate, WasmWrapper} from "./WasmWrapper"
let mockStatus = "loading"
jest.mock("@sequentech/ui-core", () => ({
    useWasm: () => ({status: mockStatus}),
    WasmStatus: {READY: "ready"},
    WasmContextProvider: ({children}: React.PropsWithChildren) => <section>{children}</section>,
}))
jest.mock("@sequentech/ui-essentials", () => ({
    Loader: () => <span>Loading verification engine</span>,
}))
it("withholds verification children until WASM is ready", () => {
    for (const status of ["loading", "error", "uninitialized"]) {
        mockStatus = status
        expect(
            renderToStaticMarkup(
                <WasmGate>
                    <strong>Verify ballot</strong>
                </WasmGate>
            )
        ).toBe("<span>Loading verification engine</span>")
    }
    mockStatus = "ready"
    expect(
        renderToStaticMarkup(
            <WasmWrapper>
                <strong>Verify ballot</strong>
            </WasmWrapper>
        )
    ).toBe("<section><strong>Verify ballot</strong></section>")
})
