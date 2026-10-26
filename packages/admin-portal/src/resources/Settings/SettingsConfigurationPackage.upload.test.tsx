/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The page's upload through the REAL drawer and import screen: the delivery
// zip is uploaded and previewed through apply_configuration_package, never
// through the election event import's dry run.
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"

const mockApply = jest.fn()
const mockGetUploadUrl = jest.fn()
const mockOtherMutation = jest.fn()
const mockFetch = jest.fn()

jest.mock("@/queries/ApplyConfigurationPackage", () => ({
    APPLY_CONFIGURATION_PACKAGE: "APPLY",
    LIST_PACKAGE_APPLICATIONS: "HISTORY",
    GET_PACKAGE_TASK: "TASK",
}))
jest.mock("@/queries/GetUploadUrl", () => ({GET_UPLOAD_URL: "UPLOAD"}))
jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    // Any other mutation (the election event import among them) is recorded.
    useMutation: (document: string) =>
        document === "APPLY"
            ? [mockApply]
            : document === "UPLOAD"
              ? [mockGetUploadUrl]
              : [(...args: unknown[]) => mockOtherMutation(document, ...args)],
    useQuery: (query: string) =>
        query === "TASK"
            ? {data: undefined, loading: false, startPolling: jest.fn(), stopPolling: jest.fn()}
            : {
                  data: {sequent_backend_tasks_execution: []},
                  refetch: jest.fn(() => Promise.resolve()),
              },
}))
jest.mock("@/providers/CreateElectionEventContextProvider", () => ({
    useCreateElectionEventStore: () => {
        throw new Error("the package upload must not use the election event import's store")
    },
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({
    useNotify: () => jest.fn(),
    SimpleForm: ({children}: React.PropsWithChildren) => children,
}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["t1"]}))
jest.mock("@/providers/AuthContextProvider", () => {
    const React = require("react")
    return {AuthContext: React.createContext({isAuthorized: () => true})}
})
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {}}),
}))
jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../ui-core/src/types/ElectionEventPresentation")
)
jest.mock("@/components/styles/FormStyles", () => ({
    FormStyles: {
        StatusBox: "div",
        ErrorMessage: "p",
        ShowProgress: () => null,
        PasswordInput: () => null,
    },
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ProblemList: () => null,
        DropFile: ({handleFiles}: {handleFiles: (files: FileList | null) => Promise<void>}) =>
            require("react").createElement("input", {
                "aria-label": "Import file",
                "type": "file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                    handleFiles(event.target.files),
            }),
        Dialog: () => null,
    }),
    {virtual: true}
)

import {SettingsConfigurationPackage} from "./SettingsConfigurationPackage"

const DOCUMENT_ID = "11111111-1111-4111-8111-111111111111"
const SHA = "ab".repeat(32)
const originalFetch = globalThis.fetch

beforeEach(() => {
    jest.clearAllMocks()
    sessionStorage.clear()
    mockGetUploadUrl.mockResolvedValue({
        data: {
            get_upload_url: {url: "https://upload.invalid/delivery.zip", document_id: DOCUMENT_ID},
        },
    })
    mockFetch.mockResolvedValue({ok: true})
    globalThis.fetch = mockFetch
    mockApply.mockResolvedValue({
        data: {apply_configuration_package: {task_execution: {id: "task-p"}}},
    })
})
afterAll(() => {
    globalThis.fetch = originalFetch
})

it("uploads the delivery through its own drawer and previews it, never the event import", async () => {
    render(<SettingsConfigurationPackage />)
    fireEvent.click(screen.getByText("configurationPackage.upload"))
    expect(screen.getByTestId("package-upload-drawer")).toBeTruthy()
    fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
        target: {value: SHA},
    })
    const file = new File(["PK synthetic delivery"], "delivery.zip", {type: "application/zip"})
    await act(async () => {
        fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
    })
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    await waitFor(() => expect((importButton as HTMLButtonElement).disabled).toBe(false))
    await act(async () => {
        fireEvent.click(importButton)
    })
    expect(mockApply).toHaveBeenCalledTimes(1)
    expect(mockApply.mock.calls[0][0].variables).toMatchObject({
        tenantId: "t1",
        documentId: DOCUMENT_ID,
        mode: "preview",
    })
    // No dry run, no import: the only mutations were the upload URL and the preview.
    expect(mockOtherMutation).not.toHaveBeenCalled()
    // The entered digest is kept with the submission, to be checked against the record.
    expect(JSON.parse(sessionStorage.getItem("configurationPackage.active.t1")!)).toMatchObject({
        taskId: "task-p",
        expectedSha: SHA,
    })
})

describe("the SHA-256 typed for the upload", () => {
    const uploadWith = async (sha: string) => {
        render(<SettingsConfigurationPackage />)
        fireEvent.click(screen.getByText("configurationPackage.upload"))
        fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
            target: {value: sha},
        })
        const file = new File(["PK synthetic delivery"], "delivery.zip", {
            type: "application/zip",
        })
        await act(async () => {
            fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
        })
        await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
        const importButton = screen.getByRole("button", {
            name: "electionEventScreen.import.import",
        })
        await waitFor(() => expect((importButton as HTMLButtonElement).disabled).toBe(false))
        await act(async () => {
            fireEvent.click(importButton)
        })
    }

    it.each([["abc"], ["sha256:" + SHA], [SHA.slice(0, 63) + "g"]])(
        "is refused visibly when malformed (%s), and nothing is previewed",
        async (sha) => {
            await uploadWith(sha)
            expect(mockApply).not.toHaveBeenCalled()
            expect(screen.getByTestId("package-error").textContent).toBe(
                "configurationPackage.shaMalformed"
            )
        }
    )

    it("is optional when only whitespace", async () => {
        await uploadWith("   ")
        expect(mockApply).toHaveBeenCalledTimes(1)
        expect(
            JSON.parse(sessionStorage.getItem("configurationPackage.active.t1")!).expectedSha
        ).toBeNull()
    })

    it("is trimmed and kept when padded", async () => {
        await uploadWith(`  ${SHA.toUpperCase()}\n`)
        expect(mockApply).toHaveBeenCalledTimes(1)
        expect(
            JSON.parse(sessionStorage.getItem("configurationPackage.active.t1")!).expectedSha
        ).toBe(SHA)
    })
})
