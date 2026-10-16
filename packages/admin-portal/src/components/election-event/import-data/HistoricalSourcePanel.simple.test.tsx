/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {HistoricalSourcePanel} from "./HistoricalSourcePanel"
const mockBuild = jest.fn()
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (_key: string, options: {defaultValue: string}) => options.defaultValue,
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ProblemList: ({report}: {report: {problems: {message: string}[]}}) => (
            <ul>
                {report.problems.map((p, i) => (
                    <li key={i}>{p.message}</li>
                ))}
            </ul>
        ),
    }),
    {virtual: true}
)
function file(name: string) {
    const source = new File(["synthetic"], name)
    Object.defineProperty(source, "arrayBuffer", {
        value: async () => new Uint8Array([1, 2, 3]).buffer,
    })
    return source
}
const sql = file("example.sql"),
    xlsx = file("example.xlsx")
const props = () => ({
    api: {buildFromHistorical: mockBuild},
    disabled: false,
    onPrepare: jest.fn(),
    onAccepted: jest.fn(async (_file: File, _hash: string) => {}),
})
const rejected = {
    summary: {sourceRows: {}, generatedRows: {}},
    output: {
        report: {
            problems: [
                {severity: "error", message: "SQL candidate source row 2: required field missing"},
            ],
        },
    },
}
beforeEach(() => {
    mockBuild.mockReset()
})
it("has no file pickers or policy controls and explains the missing source", () => {
    const p = props()
    const {container} = render(<HistoricalSourcePanel {...p} droppedFiles={[sql]} />)
    expect(container.querySelector('input[type="file"]')).toBeNull()
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument()
    expect(screen.getByText(/Drop the supplemental XLSX/)).toBeInTheDocument()
    expect(mockBuild).not.toHaveBeenCalled()
    expect(p.onAccepted).not.toHaveBeenCalled()
})
it.each([
    [sql, xlsx],
    [xlsx, sql],
])("automatically validates with the documented profile defaults", async (...files) => {
    mockBuild.mockResolvedValue(rejected)
    const p = props()
    render(<HistoricalSourcePanel {...p} droppedFiles={files} />)
    expect(await screen.findByText(/SQL candidate source row 2/)).toBeInTheDocument()
    expect(mockBuild.mock.calls[0][2]).toEqual({
        census: "configuration-only",
        ballot: "seat-plurality-at-large",
        scope: "require-complete-coverage",
        postCoverage: "require-complete-coverage",
        precinctCoverage: "require-complete-coverage",
        affiliation: "require-known-party",
        scheduleOffsetMinutes: null,
        electionMappings: [],
        areaMappings: [],
        sourceDigests: null,
    })
    expect(p.onAccepted).not.toHaveBeenCalled()
})
it("sanitizes runtime failures and allows retry with retained sources", async () => {
    mockBuild
        .mockRejectedValueOnce(new Error("private source value"))
        .mockResolvedValueOnce(rejected)
    render(<HistoricalSourcePanel {...props()} droppedFiles={[sql, xlsx]} />)
    expect(await screen.findByRole("alert")).toHaveTextContent("Converter runtime failed")
    expect(screen.queryByText("private source value")).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", {name: "Retry local validation"}))
    expect(await screen.findByText(/SQL candidate source row 2/)).toBeInTheDocument()
})
it("distinguishes a local read failure from conversion", async () => {
    const inaccessible = new File([], "inaccessible.sql")
    Object.defineProperty(inaccessible, "arrayBuffer", {
        value: async () => {
            throw new Error("private path")
        },
    })
    render(<HistoricalSourcePanel {...props()} droppedFiles={[inaccessible, xlsx]} />)
    expect(await screen.findByRole("alert")).toHaveTextContent("Could not read")
    expect(mockBuild).not.toHaveBeenCalled()
})
it("ignores an old conversion after a repeat drop", async () => {
    let finish: (value: unknown) => void = () => {}
    mockBuild
        .mockImplementationOnce(
            () =>
                new Promise((resolve) => {
                    finish = resolve
                })
        )
        .mockResolvedValueOnce(rejected)
    const p = props()
    const view = render(<HistoricalSourcePanel {...p} droppedFiles={[sql, xlsx]} />)
    await waitFor(() => expect(mockBuild).toHaveBeenCalledTimes(1))
    view.rerender(<HistoricalSourcePanel {...p} droppedFiles={[file("replacement.sql"), xlsx]} />)
    await screen.findByText(/SQL candidate source row 2/)
    await act(async () =>
        finish({
            output: {archive: {bytes: [80, 75], name: "stale.zip"}, report: {problems: []}},
            summary: {},
        })
    )
    expect(p.onAccepted).not.toHaveBeenCalled()
})
it("invalidates in-flight conversion on cancel/unmount", async () => {
    let finish: (value: unknown) => void = () => {}
    mockBuild.mockImplementation(
        () =>
            new Promise((resolve) => {
                finish = resolve
            })
    )
    const p = props()
    const view = render(<HistoricalSourcePanel {...p} droppedFiles={[sql, xlsx]} />)
    await waitFor(() => expect(mockBuild).toHaveBeenCalled())
    view.unmount()
    await act(async () => finish(rejected))
    expect(p.onAccepted).not.toHaveBeenCalled()
    render(<HistoricalSourcePanel {...p} />)
    expect(screen.queryByText(/SQL candidate source row 2/)).not.toBeInTheDocument()
})
it("prepares only the validated ZIP and its actual digest locally", async () => {
    const {webcrypto, createHash} = require("node:crypto")
    Object.defineProperty(globalThis.crypto, "subtle", {
        value: webcrypto.subtle,
        configurable: true,
    })
    mockBuild.mockResolvedValue({
        summary: {sourceRows: {}, generatedRows: {}},
        output: {archive: {name: "reviewed.zip", bytes: [80, 75, 3, 4]}, report: {problems: []}},
    })
    const p = props()
    render(<HistoricalSourcePanel {...p} droppedFiles={[sql, xlsx]} />)
    await waitFor(() => expect(p.onAccepted).toHaveBeenCalledTimes(1))
    expect(p.onAccepted).toHaveBeenCalledWith(
        expect.objectContaining({name: "reviewed.zip", type: "application/zip", size: 4}),
        createHash("sha256")
            .update(Buffer.from([80, 75, 3, 4]))
            .digest("hex")
    )
})
