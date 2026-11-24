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
const mockUnlock = jest.fn()
const mockStatus = jest.fn()
const mockStartPolling = jest.fn()
const mockStopPolling = jest.fn()
jest.mock("@/services/SignedDeliveryWorkerClient", () => ({
    ...jest.requireActual("@/services/SignedDeliveryWorkerClient"),
    unlockSignedDelivery: (...args: unknown[]) => mockUnlock(...args),
}))

// The trust section has its own tests (ConfigurationSigningSettings.test.tsx);
// here it only has to be on the page.
jest.mock("./ArchitectSignerRoster", () => ({
    ArchitectSignerRoster: () => null,
    rosterStateAllowsImport: () => true,
    useArchitectSignerRoster: () => ({
        status: {mode: "not_enabled"},
        loading: false,
        read: async () => ({mode: "not_enabled"}),
    }),
}))
jest.mock("./ArchitectApprovalPolicy", () => ({
    ArchitectApprovalPolicy: () => null,
}))
jest.mock("./ConfigurationSigningSettings", () => ({
    ConfigurationSigningSettings: ({tenantId}: {tenantId: string}) => (
        <div data-testid="configuration-signing">{tenantId}</div>
    ),
}))
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
            ? {
                  data: mockStatus(),
                  loading: false,
                  startPolling: mockStartPolling,
                  stopPolling: mockStopPolling,
              }
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
    return {AuthContext: React.createContext({isAuthorized: () => true, userId: "operator"})}
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
        disabledButtonStyle: {},
        ProblemList: () => null,
        DropFile: ({
            handleFiles,
            formatLabel,
        }: {
            handleFiles: (files: FileList | null) => Promise<void>
            formatLabel?: string
        }) =>
            require("react").createElement(
                "label",
                {"data-format": formatLabel ?? "default"},
                require("react").createElement("input", {
                    "aria-label": "Import file",
                    "type": "file",
                    "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                        handleFiles(event.target.files),
                })
            ),
        Dialog: ({
            open,
            ok,
            handleClose,
        }: {
            open: boolean
            ok: string
            handleClose: (result: boolean) => void
        }) =>
            open
                ? require("react").createElement("button", {onClick: () => handleClose(true)}, ok)
                : null,
    }),
    {virtual: true}
)

import {SettingsConfigurationPackage} from "./SettingsConfigurationPackage"
import {record} from "./configurationPackage.fixture"

const DOCUMENT_ID = "11111111-1111-4111-8111-111111111111"
const SHA = "ab".repeat(32)
const originalFetch = globalThis.fetch

beforeEach(() => {
    jest.clearAllMocks()
    mockStatus.mockReset()
    mockUnlock.mockReset()
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
    // The drop area names the delivery zip, not the import screen's default.
    expect(screen.getByLabelText("Import file").closest("label")?.dataset.format).toBe("ZIP / EZIP")
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

describe("password-encrypted signed delivery in the real upload drawer", () => {
    const uploadEncrypted = async (name = "do-not-upload-this-name.ezip", sha = "") => {
        const view = render(<SettingsConfigurationPackage />)
        fireEvent.click(screen.getByText("configurationPackage.upload"))
        if (sha) {
            fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
                target: {value: sha},
            })
        }
        const file = new File(["SEQUENT-EZIP encrypted-only"], name)
        fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
        await screen.findByRole("dialog", {name: "electionEventScreen.import.passwordDialog.title"})
        expect(mockGetUploadUrl).not.toHaveBeenCalled()
        expect(mockFetch).not.toHaveBeenCalled()
        expect(mockApply).not.toHaveBeenCalled()
        return {file, view}
    }

    const clickUnlock = async (password = "") => {
        if (password) {
            fireEvent.change(
                screen.getByLabelText("electionEventScreen.import.passwordDialog.label"),
                {
                    target: {value: password},
                }
            )
        }
        await act(async () => {
            fireEvent.click(
                screen.getByRole("button", {name: "configurationPackage.encrypted.unlock"})
            )
        })
    }

    it("requires the password with no uploads or preview on blank", async () => {
        await uploadEncrypted()
        await clickUnlock()
        expect(screen.getByText("configurationPackage.encrypted.passwordRequired")).toBeTruthy()
        expect(mockUnlock).not.toHaveBeenCalled()
        expect(mockGetUploadUrl).not.toHaveBeenCalled()
        expect(mockApply).not.toHaveBeenCalled()
    })

    it.each([
        ["decrypt", "configurationPackage.encrypted.decryptFailed"],
        ["size", "configurationPackage.encrypted.tooLarge"],
        ["unavailable", "configurationPackage.encrypted.unavailable"],
    ])("refuses worker outcome %s with no upload, preview, or apply", async (reason, key) => {
        mockUnlock.mockResolvedValue({ok: false, reason})
        await uploadEncrypted("maybe-corrupt.ezip")
        await clickUnlock("incorrect or unusable password")
        expect(screen.getByText(key)).toBeTruthy()
        expect(mockGetUploadUrl).not.toHaveBeenCalled()
        expect(mockFetch).not.toHaveBeenCalled()
        expect(mockApply).not.toHaveBeenCalled()
        expect((screen.getByTestId("package-apply") as HTMLButtonElement).disabled).toBe(true)
    })

    it("recognizes encrypted magic under a ZIP name and refuses a mismatched inner SHA", async () => {
        mockUnlock.mockResolvedValue({ok: true, bytes: new Uint8Array([80, 75]), sha256: SHA})
        await uploadEncrypted("renamed.zip", "ff".repeat(32))
        await clickUnlock("some-test-password")
        expect(screen.getByText("configurationPackage.encrypted.innerHashMismatch")).toBeTruthy()
        expect(mockGetUploadUrl).not.toHaveBeenCalled()
        expect(mockApply).not.toHaveBeenCalled()
    })

    it("uploads the exact decrypted bytes privately and sends only the pinned inner SHA to Preview", async () => {
        const bytes = new Uint8Array([80, 75, 3, 4, 255, 12])
        mockUnlock.mockResolvedValue({ok: true, bytes, sha256: SHA})
        const {file, view} = await uploadEncrypted()
        await clickUnlock("no-secret-transport")
        expect(mockUnlock).toHaveBeenCalledTimes(1)
        expect(mockUnlock.mock.calls[0][0]).toBe(file)
        await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
        expect(mockGetUploadUrl).toHaveBeenCalledWith({
            variables: {
                name: "configuration-package.zip",
                media_type: "application/zip",
                size: bytes.length,
                is_public: false,
                configuration_package: true,
            },
        })
        const upload = mockFetch.mock.calls[0][1].body as File
        const contents = await new Promise<ArrayBuffer>((resolve) => {
            const reader = new FileReader()
            reader.onload = () => resolve(reader.result as ArrayBuffer)
            reader.readAsArrayBuffer(upload)
        })
        expect(Array.from(new Uint8Array(contents))).toEqual(Array.from(bytes))
        const preview = await screen.findByRole("button", {
            name: "electionEventScreen.import.import",
        })
        await waitFor(() => expect((preview as HTMLButtonElement).disabled).toBe(false))
        await act(async () => {
            fireEvent.click(preview)
        })
        expect(mockApply.mock.calls[0][0].variables).toMatchObject({
            documentId: DOCUMENT_ID,
            mode: "preview",
            encryptedPayloadSha256: SHA,
        })
        expect(JSON.stringify(mockGetUploadUrl.mock.calls)).not.toContain("no-secret-transport")
        expect(JSON.stringify(mockFetch.mock.calls)).not.toContain("no-secret-transport")
        expect(JSON.stringify(mockApply.mock.calls)).not.toContain("no-secret-transport")
        expect(sessionStorage.getItem("configurationPackage.active.t1")).not.toContain(
            "no-secret-transport"
        )
        expect(mockOtherMutation).not.toHaveBeenCalled()

        const annotations = record("previewed")
        if (!annotations) throw new Error("Missing successful preview fixture")
        annotations.application.packageSha256 = SHA
        mockStatus.mockReturnValue({
            sequent_backend_tasks_execution: [
                {id: "task-p", execution_status: "SUCCESS", annotations},
            ],
        })
        view.rerender(<SettingsConfigurationPackage />)
        const apply = screen.getByTestId("package-apply") as HTMLButtonElement
        await waitFor(() => expect(apply.disabled).toBe(false))
        fireEvent.click(apply)
        await act(async () => {
            fireEvent.click(screen.getByTestId("package-apply-confirm"))
        })
        expect(mockApply.mock.calls[1][0].variables).toMatchObject({
            documentId: DOCUMENT_ID,
            mode: "apply",
            encryptedPayloadSha256: SHA,
        })
        expect(mockUnlock).toHaveBeenCalledTimes(1)
        expect(mockFetch).toHaveBeenCalledTimes(1)
    })
    it.each(["", SHA])(
        "discards a pending inner SHA before previewing or applying a replacement (%p)",
        async (enteredSha) => {
            const oldSha = "cd".repeat(32)
            const replacementSha = "ef".repeat(32)
            const oldDocument = "22222222-2222-4222-8222-222222222222"
            const oldPreview = record("previewed")
            if (!oldPreview) throw new Error("Missing successful preview fixture")
            oldPreview.application.packageSha256 = oldSha
            sessionStorage.setItem(
                "configurationPackage.active.t1",
                JSON.stringify({
                    taskId: "task-old",
                    mode: "preview",
                    documentId: oldDocument,
                    actor: "operator",
                    requirements: {sections: ["election-event"], census: "required"},
                    expectedSha: oldSha,
                    encryptedPayloadSha256: oldSha,
                })
            )
            mockStatus.mockReturnValue({
                sequent_backend_tasks_execution: [
                    {id: "task-old", execution_status: "SUCCESS", annotations: oldPreview},
                ],
            })
            const view = render(<SettingsConfigurationPackage />)
            const apply = screen.getByTestId("package-apply") as HTMLButtonElement
            await waitFor(() => expect(apply.disabled).toBe(false))
            fireEvent.click(screen.getByText("configurationPackage.upload"))
            fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
                target: {value: enteredSha},
            })
            const file = new File(["SEQUENT-EZIP encrypted-only"], "next.ezip")
            fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
            await screen.findByRole("dialog", {
                name: "electionEventScreen.import.passwordDialog.title",
            })
            expect(apply.disabled).toBe(true)
            mockUnlock.mockResolvedValue({ok: true, bytes: new Uint8Array([80, 75]), sha256: SHA})
            let finishPut!: (response: {ok: boolean}) => void
            mockFetch.mockImplementationOnce(
                () =>
                    new Promise((resolve) => {
                        finishPut = resolve
                    })
            )
            await clickUnlock("fixture password")
            const shaInput = screen.getByLabelText(
                "electionEventScreen.import.sha"
            ) as HTMLInputElement
            expect(shaInput.value).toBe("")
            expect(mockApply).not.toHaveBeenCalled()
            await act(async () => finishPut({ok: false}))
            expect(shaInput.value).toBe("")
            expect(mockApply).not.toHaveBeenCalled()

            const plain = new File(["PK replacement"], "replacement.zip", {type: "application/zip"})
            await act(async () => {
                fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [plain]}})
            })
            expect(shaInput.value).toBe("")
            const preview = await screen.findByRole("button", {
                name: "electionEventScreen.import.import",
            })
            await waitFor(() => expect((preview as HTMLButtonElement).disabled).toBe(false))
            fireEvent.click(preview)
            await act(async () => {
                fireEvent.click(
                    screen.getByRole("button", {name: "electionEventScreen.import.shaDialog.ok"})
                )
            })
            expect(mockApply).toHaveBeenCalledTimes(1)
            expect(mockApply.mock.calls[0][0].variables).toMatchObject({
                documentId: DOCUMENT_ID,
                mode: "preview",
            })
            expect(mockApply.mock.calls[0][0].variables).not.toHaveProperty(
                "encryptedPayloadSha256"
            )
            expect(
                JSON.parse(sessionStorage.getItem("configurationPackage.active.t1")!).expectedSha
            ).toBeNull()
            // A stale response from the recalled task is not this upload's preview.
            expect(apply.disabled).toBe(true)
            const replacementPreview = record("previewed")
            if (!replacementPreview) throw new Error("Missing successful preview fixture")
            replacementPreview.application.packageSha256 = replacementSha
            mockStatus.mockReturnValue({
                sequent_backend_tasks_execution: [
                    {id: "task-p", execution_status: "SUCCESS", annotations: replacementPreview},
                ],
            })
            view.rerender(<SettingsConfigurationPackage />)
            await waitFor(() => expect(apply.disabled).toBe(false))
            fireEvent.click(apply)
            await act(async () => {
                fireEvent.click(screen.getByTestId("package-apply-confirm"))
            })
            expect(mockApply).toHaveBeenCalledTimes(2)
            expect(mockApply.mock.calls[1][0].variables).toMatchObject({
                documentId: DOCUMENT_ID,
                mode: "apply",
            })
            expect(mockApply.mock.calls[1][0].variables).not.toHaveProperty(
                "encryptedPayloadSha256"
            )
        }
    )

    it("does not restore a recalled encrypted preview from a different task's result", async () => {
        const other = record("previewed")
        if (!other) throw new Error("Missing successful preview fixture")
        other.application.packageSha256 = SHA
        sessionStorage.setItem(
            "configurationPackage.active.t1",
            JSON.stringify({
                taskId: "recalled-task",
                mode: "preview",
                documentId: DOCUMENT_ID,
                actor: "operator",
                requirements: {sections: ["election-event"], census: "required"},
                expectedSha: SHA,
                encryptedPayloadSha256: SHA,
            })
        )
        mockStatus.mockReturnValue({
            sequent_backend_tasks_execution: [
                {id: "different-task", execution_status: "SUCCESS", annotations: other},
            ],
        })
        render(<SettingsConfigurationPackage />)
        const apply = screen.getByTestId("package-apply") as HTMLButtonElement
        expect(apply.disabled).toBe(true)
        expect(mockApply).not.toHaveBeenCalled()
    })
})
