/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {HistoricalSourcePanel} from "./HistoricalSourcePanel"
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        loaded: true,
        globalSettings: {APP_VERSION: "v11.0.0-rc.0"},
    }),
}))
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
    return [sql, xlsx]
}
beforeEach(() => {
    mockBuild.mockReset()
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
            droppedFiles={inputs()}
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    await waitFor(() => expect(mockBuild).toHaveBeenCalled())
    expect(await screen.findByText("Duplicate workbook post key")).toBeInTheDocument()
    expect(accepted).not.toHaveBeenCalled()
})

it("names the source row, field and dangling reference, and never accepts the conversion", async () => {
    const accepted = jest.fn()
    mockBuild.mockResolvedValue({
        summary: {
            sourceRows: {candidate: 2},
            generatedRows: {},
            excludedVoters: 0,
            outsideScopeCandidates: 0,
            opaqueBinaryValues: 0,
        },
        output: {
            report: {
                problems: [
                    {
                        severity: "error",
                        code: "dangling_reference",
                        path: "SQL table candidate row 2 column POST_CODE",
                        message: "contest C99 does not exist in the source contest table",
                        details: {reference: "C99", target: "contest.POSTCODE"},
                    },
                ],
            },
        },
    })
    render(
        <HistoricalSourcePanel
            droppedFiles={inputs()}
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    expect(
        await screen.findByText(
            "SQL table candidate row 2 column POST_CODE: contest C99 does not exist in the source contest table"
        )
    ).toBeInTheDocument()
    expect(accepted).not.toHaveBeenCalled()
})

it("keeps local source files after a runtime failure and does not reveal exception values", async () => {
    const accepted = jest.fn()
    mockBuild.mockRejectedValue(new Error("synthetic-private-value"))
    render(
        <HistoricalSourcePanel
            droppedFiles={inputs()}
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    expect(await screen.findByRole("alert")).toHaveTextContent("Converter runtime failed")
    expect(screen.queryByText("synthetic-private-value")).not.toBeInTheDocument()
    expect(accepted).not.toHaveBeenCalled()
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
            droppedFiles={inputs()}
            api={{buildFromHistorical: mockBuild}}
            disabled={false}
            onPrepare={jest.fn()}
            onAccepted={accepted}
        />
    )
    await waitFor(() => expect(mockBuild).toHaveBeenCalled())
    expect(screen.getByRole("status")).toHaveTextContent("Validating SQL and workbook locally")
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
    expect(accepted).not.toHaveBeenCalled()
})

it("passes only the validated ZIP and its actual SHA-256 after local validation, without a backend request", async () => {
    const {webcrypto, createHash} = require("node:crypto")
    const original = Object.getOwnPropertyDescriptor(globalThis.crypto, "subtle")
    Object.defineProperty(globalThis.crypto, "subtle", {
        value: webcrypto.subtle,
        configurable: true,
    })
    try {
        const accepted = jest.fn(async () => {})
        const bytes = [80, 75, 3, 4]
        mockBuild.mockResolvedValue({
            summary: {sourceRows: {}, generatedRows: {}, excludedVoters: 1, opaqueBinaryValues: 0},
            output: {archive: {name: "reviewed.zip", bytes}, report: {problems: []}},
        })
        render(
            <HistoricalSourcePanel
                droppedFiles={inputs()}
                api={{buildFromHistorical: mockBuild}}
                disabled={false}
                onPrepare={jest.fn()}
                onAccepted={accepted}
            />
        )
        await waitFor(() => expect(accepted).toHaveBeenCalledTimes(1))
        const [file, digest] = accepted.mock.calls[0] as unknown as [File, string]
        expect(file.name).toBe("reviewed.zip")
        expect(file.type).toBe("application/zip")
        expect(file.size).toBe(bytes.length)
        expect(digest).toBe(createHash("sha256").update(Buffer.from(bytes)).digest("hex"))
    } finally {
        if (original) Object.defineProperty(globalThis.crypto, "subtle", original)
        else Reflect.deleteProperty(globalThis.crypto, "subtle")
    }
})
