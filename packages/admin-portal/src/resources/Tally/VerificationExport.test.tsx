/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import "@testing-library/jest-dom"
import {VerificationExport} from "./VerificationExport"

const mockQuery = jest.fn()
const mockExport = jest.fn()
const mockDownload = jest.fn()
const mockLoad = jest.fn()
const mockAuthorized = jest.fn((..._args: unknown[]) => true)
jest.mock("@apollo/client", () => ({
    gql: (source: TemplateStringsArray) => source.join(""),
    useQuery: (...args: unknown[]) => mockQuery(...args),
    useMutation: () => [mockExport],
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (_key: string, options: {defaultValue: string}) => options.defaultValue,
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    downloadUrl: (...args: unknown[]) => mockDownload(...args),
}))
jest.mock("@/services/BrowserTrustee", () => ({loadBrowserTrustee: () => mockLoad()}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (...args: unknown[]) => mockAuthorized(...args),
    }),
}))

const event = "20000000-0000-4000-8000-000000000002"
const keys = "30000000-0000-4000-8000-000000000003"
const session = "40000000-0000-4000-8000-000000000004"
const parent = {
    id: "50000000-0000-4000-8000-000000000005",
    tenant_id: "tenant",
    election_event_id: event,
    name: `dkg_${keys.replace(/-/g, "")}`,
    parent_id: null,
    keys_ceremony_id: keys,
    tally_session_id: null,
    batch: null,
    manager_message: "\\x0102ff",
}
const board = {
    ...parent,
    id: "ffffffff-ffff-4fff-bfff-ffffffffffff",
    parent_id: parent.id,
    name: `tally_${session.replace(/-/g, "")}_0`,
    tally_session_id: session,
    batch: 0,
}
const hash = "ab".repeat(64)
const props = {eventId: event, tallySessionId: session, completed: true}
beforeEach(() => {
    jest.clearAllMocks()
    mockAuthorized.mockReturnValue(true)
    mockQuery.mockReturnValue({
        data: {sequent_backend_protocol_board: [parent, board]},
        loading: false,
    })
    mockLoad.mockResolvedValue({WasmVerifier: {configurationHash: () => hash}})
    mockExport.mockResolvedValue({
        data: {
            export_verification_transcript: {
                document_id: "document",
                url: "https://vote.example.test/private/transcript",
            },
        },
    })
    mockDownload.mockResolvedValue(undefined)
})

it("shows independent full anchors and exports only the chosen board via its authorized action", async () => {
    render(<VerificationExport {...props} />)
    const button = await screen.findByRole("button", {name: "Download verification data"})
    expect(screen.getByLabelText("Configuration hash")).toHaveValue(hash)
    expect(screen.getByLabelText("Tally ID")).toHaveValue(
        BigInt("0xffffffffffff4fffbfffffffffffffff").toString()
    )
    expect(mockQuery.mock.calls[0][1]).toMatchObject({
        variables: {tenantId: "tenant", eventId: event},
        context: {headers: {"x-hasura-role": "admin-ceremony"}},
    })
    fireEvent.click(button)
    await waitFor(() =>
        expect(mockDownload).toHaveBeenCalledWith(
            "https://vote.example.test/private/transcript",
            `verification-${board.id}.json`
        )
    )
    expect(mockExport).toHaveBeenCalledWith({
        variables: {eventId: event, boardId: board.id},
        context: {headers: {"x-hasura-role": "admin-ceremony"}},
    })
})

it.each(["incomplete", "unauthorized"])(
    "does not load anchors or offer download when %s",
    (reason) => {
        if (reason === "unauthorized") mockAuthorized.mockReturnValue(false)
        render(<VerificationExport {...props} completed={reason !== "incomplete"} />)
        expect(screen.queryByRole("button")).not.toBeInTheDocument()
        expect(mockLoad).not.toHaveBeenCalled()
        expect(mockQuery.mock.calls[0][1].skip).toBe(true)
    }
)

it("shows a fixed error without exposing a failed backend response or downloading", async () => {
    mockExport.mockRejectedValue(new Error("private response details"))
    render(<VerificationExport {...props} />)
    fireEvent.click(await screen.findByRole("button", {name: "Download verification data"}))
    expect(await screen.findByText("Verification data could not be downloaded.")).toBeVisible()
    expect(screen.queryByText("private response details")).not.toBeInTheDocument()
    expect(mockDownload).not.toHaveBeenCalled()
})

it("refuses malformed independent anchors without offering a transcript", async () => {
    mockLoad.mockResolvedValue({
        WasmVerifier: {
            configurationHash: () => {
                throw new Error("malformed")
            },
        },
    })
    render(<VerificationExport {...props} />)
    expect(await screen.findByText("Verification anchors could not be loaded.")).toBeVisible()
    expect(screen.queryByRole("button")).not.toBeInTheDocument()
})

it("does not download a pending prior board after navigation to another tally", async () => {
    let finish: (value: unknown) => void = () => undefined
    mockExport.mockReturnValue(
        new Promise((resolve) => {
            finish = resolve
        })
    )
    const {rerender} = render(<VerificationExport {...props} />)
    fireEvent.click(await screen.findByRole("button", {name: "Download verification data"}))
    rerender(<VerificationExport {...props} tallySessionId="another-session" />)
    finish({
        data: {
            export_verification_transcript: {
                document_id: "document",
                url: "https://vote.example.test/private/transcript",
            },
        },
    })
    await waitFor(() => expect(screen.queryByRole("button")).not.toBeInTheDocument())
    expect(mockDownload).not.toHaveBeenCalled()
})

it("keeps sibling batch anchors and selected export IDs separate", async () => {
    const sibling = {
        ...board,
        id: "ffffffff-ffff-4fff-bfff-fffffffffffe",
        batch: 1,
        name: `tally_${session.replace(/-/g, "")}_1`,
    }
    mockQuery.mockReturnValue({
        data: {sequent_backend_protocol_board: [parent, board, sibling]},
        loading: false,
    })
    render(<VerificationExport {...props} />)
    const label = await screen.findByText(sibling.name)
    const row = label.closest("[data-verification-board]") as HTMLElement
    expect(within(row).getByLabelText("Tally ID")).toHaveValue(
        BigInt("0xffffffffffff4fffbffffffffffffffe").toString()
    )
    fireEvent.click(within(row).getByRole("button", {name: "Download verification data"}))
    await waitFor(() =>
        expect(mockExport).toHaveBeenCalledWith({
            variables: {eventId: event, boardId: sibling.id},
            context: {headers: {"x-hasura-role": "admin-ceremony"}},
        })
    )
    expect(mockDownload).toHaveBeenCalledWith(
        "https://vote.example.test/private/transcript",
        `verification-${sibling.id}.json`
    )
})

it.each([
    {data: {export_verification_transcript: null}},
    {data: {export_verification_transcript: {document_id: "document", url: "javascript:void(0)"}}},
    {
        data: {
            export_verification_transcript: {
                document_id: "document",
                url: "https://vote.example.test/file",
            },
        },
        errors: [{message: "failed"}],
    },
])("does not download a missing, unsafe or failed export", async (response) => {
    mockExport.mockResolvedValue(response)
    render(<VerificationExport {...props} />)
    fireEvent.click(await screen.findByRole("button", {name: "Download verification data"}))
    expect(await screen.findByText("Verification data could not be downloaded.")).toBeVisible()
    expect(mockDownload).not.toHaveBeenCalled()
})
