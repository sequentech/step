/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
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
function inputs() {
    const sql = new File(["synthetic SQL"], "example.sql")
    const xlsx = new File(["synthetic XLSX"], "example.xlsx")
    Object.defineProperty(sql, "arrayBuffer", {value: async () => new Uint8Array([1, 2, 3]).buffer})
    Object.defineProperty(xlsx, "arrayBuffer", {
        value: async () => new Uint8Array([4, 5, 6]).buffer,
    })
    fireEvent.change(screen.getByLabelText("Historical SQL"), {target: {files: [sql]}})
    fireEvent.change(screen.getByLabelText("Historical workbook"), {target: {files: [xlsx]}})
}
async function select(label: string, choice: string) {
    fireEvent.mouseDown(screen.getByRole("combobox", {name: label}))
    fireEvent.click(await screen.findByRole("option", {name: choice}))
}
beforeEach(() => {
    mockBuild.mockReset()
})
it("does not convert or upload until the operator selects explicit decisions", () => {
    const accepted = jest.fn()
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    inputs()
    expect(screen.getByRole("button", {name: "Convert and validate locally"})).toBeDisabled()
    expect(mockBuild).not.toHaveBeenCalled()
    expect(accepted).not.toHaveBeenCalled()
})
it("shows rejected source counts and diagnostics without calling upload", async () => {
    const accepted = jest.fn()
    mockBuild.mockReturnValue({
        summary: {
            sqlSha256: "a".repeat(64),
            workbookSha256: "b".repeat(64),
            sourceRows: {voters: 80},
            generatedRows: {},
            excludedVoters: 80,
            outsideScopeCandidates: 0,
            opaqueBinaryValues: 50,
        },
        output: {report: {problems: [{severity: "error", message: "Duplicate workbook post key"}]}},
    })
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    inputs()
    await select(
        "Census decision",
        "Configuration only: exclude voters; eligibility requires separate review"
    )
    await select(
        "Reviewed ballot policy",
        "Plurality at large: seat count sets maximum choices and winners; minimum 0"
    )
    await select(
        "Candidate scope decision",
        "Use configured seats only; report candidates outside that scope"
    )
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    await waitFor(() => expect(mockBuild).toHaveBeenCalled())
    expect(await screen.findByText("Duplicate workbook post key")).toBeInTheDocument()
    expect(screen.getByRole("button", {name: "Accept reviewed package and upload"})).toBeDisabled()
    expect(accepted).not.toHaveBeenCalled()
})

it("binds explicit seat associations to the inspected hashes and clears them when source files change", async () => {
    const summary = {
        sqlSha256: "a".repeat(64),
        workbookSha256: "b".repeat(64),
        sourceRows: {seat: 1},
        generatedRows: {},
        excludedVoters: 0,
        outsideScopeCandidates: 0,
        opaqueBinaryValues: 0,
        excludedPollingCenters: 0,
        electionContexts: [{workbookRow: 2, centerRows: [1]}],
        seats: [{sourceRow: 1, candidateRows: 2, eligibleAmount: "1"}],
        precincts: [],
        areaWorkbookRows: [],
    }
    mockBuild.mockReturnValue({summary, output: {report: {problems: []}}})
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={jest.fn()}
        />
    )
    inputs()
    await select(
        "Census decision",
        "Configuration only: exclude voters; eligibility requires separate review"
    )
    await select(
        "Reviewed ballot policy",
        "Plurality at large: seat count sets maximum choices and winners; minimum 0"
    )
    await select("Candidate scope decision", "Require every candidate to match a configured seat")
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    const checkbox = await screen.findByRole("checkbox", {
        name: "SQL seat row 1: 2 candidates, seat count 1",
    })
    fireEvent.click(checkbox)
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    await waitFor(() => expect(mockBuild).toHaveBeenCalledTimes(2))
    expect(mockBuild.mock.calls[1][2]).toMatchObject({
        electionMappings: [{workbookRow: 2, seatRows: [1]}],
        sourceDigests: {sqlSha256: summary.sqlSha256, workbookSha256: summary.workbookSha256},
    })
    fireEvent.change(screen.getByLabelText("Historical SQL"), {
        target: {files: [new File(["changed"], "changed.sql")]},
    })
    expect(
        screen.queryByRole("checkbox", {name: "SQL seat row 1: 2 candidates, seat count 1"})
    ).not.toBeInTheDocument()
})

async function reviewedDecisions() {
    await select(
        "Census decision",
        "Configuration only: exclude voters; eligibility requires separate review"
    )
    await select(
        "Reviewed ballot policy",
        "Plurality at large: seat count sets maximum choices and winners; minimum 0"
    )
    await select("Candidate scope decision", "Require every candidate to match a configured seat")
}
it("keeps local source files after a runtime failure and does not reveal exception values", async () => {
    const accepted = jest.fn()
    mockBuild.mockRejectedValue(new Error("synthetic-private-value"))
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    inputs()
    await reviewedDecisions()
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    expect(await screen.findByRole("alert")).toHaveTextContent("Converter runtime failed")
    expect(screen.queryByText("synthetic-private-value")).not.toBeInTheDocument()
    expect((screen.getByLabelText("Historical SQL") as HTMLInputElement).files?.[0].name).toBe(
        "example.sql"
    )
    expect((screen.getByLabelText("Historical workbook") as HTMLInputElement).files?.[0].name).toBe(
        "example.xlsx"
    )
    expect(accepted).not.toHaveBeenCalled()
})
it("distinguishes local file access failure before calling the converter", async () => {
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={jest.fn()}
        />
    )
    inputs()
    const inaccessible = new File(["synthetic"], "inaccessible.sql")
    Object.defineProperty(inaccessible, "arrayBuffer", {
        value: async () => {
            throw new Error("synthetic-private-path")
        },
    })
    fireEvent.change(screen.getByLabelText("Historical SQL"), {target: {files: [inaccessible]}})
    await reviewedDecisions()
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    expect(await screen.findByRole("alert")).toHaveTextContent(
        "Could not read one of the local source files"
    )
    expect(mockBuild).not.toHaveBeenCalled()
    expect(screen.queryByText("synthetic-private-path")).not.toBeInTheDocument()
})
it("waits for local asynchronous validation and never accepts a rejected result", async () => {
    const accepted = jest.fn()
    let finish: ((value: unknown) => void) | undefined
    mockBuild.mockImplementation(
        () =>
            new Promise((resolve) => {
                finish = resolve
            })
    )
    render(
        <HistoricalSourcePanel
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    inputs()
    await reviewedDecisions()
    fireEvent.click(screen.getByRole("button", {name: "Convert and validate locally"}))
    await waitFor(() => expect(mockBuild).toHaveBeenCalled())
    expect(screen.getByRole("button", {name: "Convert and validate locally"})).toBeDisabled()
    expect(accepted).not.toHaveBeenCalled()
    finish?.({
        summary: {sourceRows: {}, generatedRows: {}},
        output: {
            report: {
                problems: [
                    {
                        severity: "error",
                        message: "SQL candidate source row 2: required field missing",
                    },
                ],
            },
        },
    })
    expect(
        await screen.findByText("SQL candidate source row 2: required field missing")
    ).toBeInTheDocument()
    expect(screen.getByRole("button", {name: "Accept reviewed package and upload"})).toBeDisabled()
    expect(accepted).not.toHaveBeenCalled()
})
