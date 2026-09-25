/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import "@testing-library/jest-dom"
import {ImportScreen} from "./ImportScreen"

const mockGetUploadUrl = jest.fn()
const mockNotify = jest.fn()
const mockFetch = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: jest.fn(),
    useMutation: () => [mockGetUploadUrl],
}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => ({
    useNotify: () => mockNotify,
    SimpleForm: ({children}: React.PropsWithChildren) => children,
}))
jest.mock("@/components/styles/FormStyles", () => ({
    FormStyles: {
        StatusBox: "div",
        ErrorMessage: "p",
        ShowProgress: () => require("react").createElement("div", {role: "progressbar"}),
        PasswordInput: ({
            inputProps,
            onChange,
        }: {
            inputProps: React.InputHTMLAttributes<HTMLInputElement>
            onChange: React.ChangeEventHandler<HTMLInputElement>
        }) => require("react").createElement("input", {...inputProps, type: "password", onChange}),
    },
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        DropFile: ({handleFiles}: {handleFiles: (files: FileList | null) => Promise<void>}) =>
            require("react").createElement("input", {
                "aria-label": "Import file",
                "type": "file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) =>
                    handleFiles(event.target.files),
            }),
        Dialog: ({
            open,
            title,
            ok,
            cancel,
            handleClose,
            children,
        }: React.PropsWithChildren<{
            open: boolean
            title: string
            ok: string
            cancel?: string
            handleClose: (confirmed: boolean) => void
        }>) =>
            open
                ? require("react").createElement(
                      "div",
                      {"role": "dialog", "aria-label": title},
                      children,
                      require("react").createElement(
                          "button",
                          {onClick: () => handleClose(true)},
                          ok
                      ),
                      cancel &&
                          require("react").createElement(
                              "button",
                              {onClick: () => handleClose(false)},
                              cancel
                          )
                  )
                : null,
    }),
    {virtual: true}
)

const UPLOAD_URL = "https://upload.invalid/event.json"
const DOCUMENT_ID = "11111111-1111-4111-8111-111111111111"
const CHECKSUM = "01".repeat(32)
const originalFetch = globalThis.fetch

beforeEach(() => {
    jest.clearAllMocks()
    mockGetUploadUrl.mockResolvedValue({
        data: {get_upload_url: {url: UPLOAD_URL, document_id: DOCUMENT_ID}},
    })
    mockFetch.mockResolvedValue({ok: true})
    globalThis.fetch = mockFetch
})
afterAll(() => {
    globalThis.fetch = originalFetch
})

function renderImport() {
    const uploadCallback = jest.fn(async () => {})
    const doImport = jest.fn(async () => {})
    const doCancel = jest.fn()
    render(
        <ImportScreen
            uploadCallback={uploadCallback}
            doImport={doImport}
            doCancel={doCancel}
            errors={null}
        />
    )
    return {uploadCallback, doImport, doCancel}
}

function uploadFile() {
    const file = new File(['{"elections":[]}'], "event.json", {type: "application/json"})
    fireEvent.change(screen.getByLabelText("Import file"), {target: {files: [file]}})
    return file
}

it("uploads the private file before enabling import and forwards its integrity check", async () => {
    const callbacks = renderImport()
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeDisabled()
    fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
        target: {value: CHECKSUM},
    })
    let finishUpload: ((response: {ok: boolean}) => void) | undefined
    mockFetch.mockImplementation(
        () =>
            new Promise((resolve) => {
                finishUpload = resolve
            })
    )
    const file = uploadFile()
    await waitFor(() => expect(mockFetch).toHaveBeenCalledTimes(1))
    expect(importButton).toBeDisabled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(mockGetUploadUrl).toHaveBeenCalledWith({
        variables: {name: "event.json", media_type: "application/json", size: 16, is_public: false},
    })
    expect(mockFetch).toHaveBeenCalledWith(UPLOAD_URL, {
        method: "PUT",
        headers: {"Content-Type": "application/json"},
        body: file,
    })
    await act(async () => finishUpload?.({ok: true}))
    expect(callbacks.uploadCallback).toHaveBeenCalledWith(DOCUMENT_ID, "", CHECKSUM)
    expect(importButton).toBeEnabled()
    fireEvent.click(importButton)
    await waitFor(() => expect(callbacks.doImport).toHaveBeenCalledWith(DOCUMENT_ID, CHECKSUM, ""))
    expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadSuccess", {
        type: "success",
    })
})

it.each([
    ["HTTP 503", () => mockFetch.mockResolvedValue({ok: false, status: 503})],
    ["connection failure", () => mockFetch.mockRejectedValue(new TypeError("connection lost"))],
])(
    "keeps failed %s uploads unavailable for import and permits retry",
    async (_reason, injectFailure) => {
        const callbacks = renderImport()
        injectFailure()
        uploadFile()
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
                type: "error",
            })
        )
        expect(callbacks.uploadCallback).not.toHaveBeenCalled()
        expect(callbacks.doImport).not.toHaveBeenCalled()
        expect(
            screen.getByRole("button", {name: "electionEventScreen.import.import"})
        ).toBeDisabled()
        expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
        mockFetch.mockResolvedValue({ok: true})
        uploadFile()
        await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
        expect(
            screen.getByRole("button", {name: "electionEventScreen.import.import"})
        ).toBeEnabled()
    }
)

it("releases the form when no upload URL is returned and allows a later upload", async () => {
    const callbacks = renderImport()
    mockGetUploadUrl.mockResolvedValueOnce({data: {get_upload_url: {document_id: DOCUMENT_ID}}})
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(mockFetch).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.cancel"})).toBeEnabled()
    expect(screen.getByRole("button", {name: "electionEventScreen.import.import"})).toBeDisabled()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
})

it("reports an upload URL request failure without issuing a PUT", async () => {
    const callbacks = renderImport()
    mockGetUploadUrl.mockRejectedValue(new Error("service unavailable"))
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(mockFetch).not.toHaveBeenCalled()
    expect(callbacks.uploadCallback).not.toHaveBeenCalled()
    expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()
})

it.each([
    [
        "missing URL",
        () =>
            mockGetUploadUrl.mockResolvedValueOnce({
                data: {get_upload_url: {document_id: "second-document"}},
            }),
    ],
    ["HTTP 503", () => mockFetch.mockResolvedValueOnce({ok: false, status: 503})],
    ["connection failure", () => mockFetch.mockRejectedValueOnce(new TypeError("connection lost"))],
    [
        "URL service failure",
        () => mockGetUploadUrl.mockRejectedValueOnce(new Error("service unavailable")),
    ],
])(
    "invalidates the previous uploaded document when a replacement fails with %s",
    async (_reason, injectFailure) => {
        const callbacks = renderImport()
        const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
        fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
            target: {value: CHECKSUM},
        })
        uploadFile()
        await waitFor(() =>
            expect(callbacks.uploadCallback).toHaveBeenCalledWith(DOCUMENT_ID, "", CHECKSUM)
        )
        expect(importButton).toBeEnabled()

        const replacementChecksum = "23".repeat(32)
        fireEvent.change(screen.getByLabelText("electionEventScreen.import.sha"), {
            target: {value: replacementChecksum},
        })
        injectFailure()
        uploadFile()
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
                type: "error",
            })
        )
        expect(importButton).toBeDisabled()
        fireEvent.click(importButton)
        expect(callbacks.doImport).not.toHaveBeenCalled()
        expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1)
        expect(screen.getByLabelText("electionEventScreen.import.sha")).toBeEnabled()

        mockGetUploadUrl.mockResolvedValueOnce({
            data: {get_upload_url: {url: UPLOAD_URL, document_id: "replacement-document"}},
        })
        uploadFile()
        await waitFor(() =>
            expect(callbacks.uploadCallback).toHaveBeenCalledWith(
                "replacement-document",
                "",
                replacementChecksum
            )
        )
        fireEvent.click(importButton)
        await waitFor(() =>
            expect(callbacks.doImport).toHaveBeenCalledWith(
                "replacement-document",
                replacementChecksum,
                ""
            )
        )
        expect(callbacks.doImport).toHaveBeenCalledTimes(1)
    }
)

it("keeps import unavailable when the replacement upload callback fails", async () => {
    const callbacks = renderImport()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeEnabled()
    callbacks.uploadCallback.mockRejectedValueOnce(new Error("invalid replacement document"))
    uploadFile()
    await waitFor(() =>
        expect(mockNotify).toHaveBeenCalledWith("electionEventScreen.import.fileUploadError", {
            type: "error",
        })
    )
    expect(importButton).toBeDisabled()
    expect(callbacks.doImport).not.toHaveBeenCalled()
})

it("invalidates a previous document as soon as a replacement encrypted file is selected", async () => {
    const callbacks = renderImport()
    uploadFile()
    await waitFor(() => expect(callbacks.uploadCallback).toHaveBeenCalledTimes(1))
    const importButton = screen.getByRole("button", {name: "electionEventScreen.import.import"})
    expect(importButton).toBeEnabled()
    fireEvent.change(screen.getByLabelText("Import file"), {
        target: {files: [new File(["encrypted"], "event.ezip")]},
    })
    expect(screen.getByRole("dialog")).toBeInTheDocument()
    expect(importButton).toBeDisabled()
    expect(mockGetUploadUrl).toHaveBeenCalledTimes(1)
    expect(callbacks.doImport).not.toHaveBeenCalled()
})
