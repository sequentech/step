/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {HistoricalImportPlugin} from "./HistoricalImportPlugin"
import {createHistoricalImportConverter} from "@/services/HistoricalImportWorkerClient"
jest.mock("@/services/HistoricalImportWorkerClient", () => ({
    createHistoricalImportConverter: jest.fn(),
}))
jest.mock("./HistoricalSourcePanel", () => ({
    HistoricalSourcePanel: ({disabled}: {disabled: boolean}) => (
        <>
            <p>Historical source controls</p>
            <input type="file" aria-label="Retained sources" disabled={disabled} />
        </>
    ),
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (_key: string, options: {defaultValue: string}) => options.defaultValue,
    }),
}))
const props = {disabled: false, onPrepare: jest.fn(), onAccepted: jest.fn()}
beforeEach(() => {
    jest.clearAllMocks()
})
it("hides the feature and loads nothing when configuration is absent", () => {
    const {container} = render(<HistoricalImportPlugin {...props} />)
    expect(container).toBeEmptyDOMElement()
    expect(createHistoricalImportConverter).not.toHaveBeenCalled()
})
it("shows the configured plugin after loading", async () => {
    ;(createHistoricalImportConverter as jest.Mock).mockReturnValue({
        ready: Promise.resolve({buildFromHistorical: jest.fn()}),
        dispose: jest.fn(),
    })
    render(<HistoricalImportPlugin {...props} baseUrl="/wasm/converter" />)
    expect(screen.getByRole("status")).toHaveTextContent("Loading configured")
    expect(await screen.findByText("Historical source controls")).toBeInTheDocument()
    expect(createHistoricalImportConverter).toHaveBeenCalledWith("/wasm/converter")
})
it("shows a clear load error without source conversion controls", async () => {
    ;(createHistoricalImportConverter as jest.Mock).mockReturnValue({
        ready: Promise.reject(new Error("Incompatible converter ABI")),
        dispose: jest.fn(),
    })
    render(<HistoricalImportPlugin {...props} baseUrl="/wasm/bad" />)
    expect(await screen.findByRole("alert")).toHaveTextContent("Incompatible converter ABI")
    expect(screen.queryByText("Historical source controls")).not.toBeInTheDocument()
})
it("removes old controls immediately when configured assets change", async () => {
    ;(createHistoricalImportConverter as jest.Mock).mockReturnValue({
        ready: Promise.resolve({buildFromHistorical: jest.fn()}),
        dispose: jest.fn(),
    })
    const view = render(<HistoricalImportPlugin {...props} baseUrl="/wasm/first" />)
    await screen.findByText("Historical source controls")
    ;(createHistoricalImportConverter as jest.Mock).mockReturnValue({
        ready: new Promise(() => {}),
        dispose: jest.fn(),
    })
    view.rerender(<HistoricalImportPlugin {...props} baseUrl="/wasm/second" />)
    expect(screen.queryByText("Historical source controls")).not.toBeInTheDocument()
    await waitFor(() =>
        expect(createHistoricalImportConverter).toHaveBeenCalledWith("/wasm/second")
    )
})

it("offers reload after a load failure and recovers", async () => {
    const dispose = jest.fn()
    ;(createHistoricalImportConverter as jest.Mock)
        .mockReturnValueOnce({ready: Promise.reject(new Error("Asset load failed")), dispose})
        .mockReturnValueOnce({
            ready: Promise.resolve({buildFromHistorical: jest.fn()}),
            dispose: jest.fn(),
        })
    render(<HistoricalImportPlugin {...props} baseUrl="/wasm/converter" />)
    await screen.findByRole("alert")
    fireEvent.click(screen.getByRole("button", {name: "Reload converter"}))
    expect(await screen.findByText("Historical source controls")).toBeInTheDocument()
    expect(dispose).toHaveBeenCalled()
    expect(screen.queryByRole("button", {name: "Reload converter"})).not.toBeInTheDocument()
})
