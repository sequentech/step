/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {CreateSupportMaterial} from "./CreateSupportMaterial"

const mockNotify = jest.fn()
const mockGetUploadUrl = jest.fn()
const mockFetch = jest.fn()
let mockHandled: Promise<void> | undefined

jest.mock("@apollo/client", () => ({useMutation: () => [mockGetUploadUrl]}))
jest.mock("@/queries/GetUploadUrl", () => ({GET_UPLOAD_URL: "GetUploadUrl"}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("react-admin", () => {
    const passThrough = ({children}: React.PropsWithChildren) => children
    return {
        Create: passThrough,
        SimpleForm: passThrough,
        Toolbar: () => null,
        SaveButton: () => null,
        TextInput: () => null,
        BooleanInput: () => null,
        ValidationError: () => null,
        useNotify: () => mockNotify,
        useRefresh: () => jest.fn(),
    }
})
jest.mock("react-hook-form", () => ({useFormState: () => ({errors: {}})}))
jest.mock("react-admin-json-view", () => ({JsonInput: () => null}))
jest.mock("../../components/styles/PageHeaderStyles", () => ({
    PageHeaderStyles: {Wrapper: "div", Title: "div", SubTitle: "div"},
}))
jest.mock("@/components/Tabs", () => ({Tabs: () => null}))
jest.mock("./MaterialLanguageFields", () => ({MaterialLanguageFields: () => null}))
jest.mock("./AudioInstructionsFields", () => ({
    AudioInstructionsFields: () => null,
    withAudioInstructions: (value: unknown) => value,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        DropFile: ({handleFiles}: {handleFiles: (files: FileList | null) => Promise<void>}) =>
            require("react").createElement("input", {
                "aria-label": "Material file",
                "type": "file",
                "onChange": (event: React.ChangeEvent<HTMLInputElement>) => {
                    mockHandled = handleFiles(event.target.files)
                },
            }),
    }),
    {virtual: true}
)

const NAME_TEXT =
    "A document name cannot contain / \\ ; # % ? or a control character. Rename it and try again."
const originalFetch = globalThis.fetch

beforeEach(() => {
    jest.clearAllMocks()
    mockHandled = undefined
    globalThis.fetch = mockFetch
})
afterAll(() => {
    globalThis.fetch = originalFetch
})

const chooseFile = (name = "guide #1.pdf") => {
    render(<CreateSupportMaterial record={{id: "event", tenant_id: "tenant"}} />)
    const file = new File(["%PDF"], name, {type: "application/pdf"})
    fireEvent.change(screen.getByLabelText("Material file"), {target: {files: [file]}})
}

it("explains a refused file name and uploads nothing", async () => {
    mockGetUploadUrl.mockRejectedValueOnce(
        Object.assign(new Error(NAME_TEXT), {
            graphQLErrors: [{message: NAME_TEXT, extensions: {code: "invalid-document-name"}}],
        })
    )
    chooseFile()
    await waitFor(() => expect(mockHandled).toBeDefined())
    await expect(mockHandled).resolves.toBeUndefined()
    expect(mockNotify).toHaveBeenCalledTimes(1)
    expect(mockNotify).toHaveBeenCalledWith("refusals.documentName", {type: "error"})
    expect(mockFetch).not.toHaveBeenCalled()
})

it("shows the form's own message when the upload address can't be had", async () => {
    mockGetUploadUrl.mockRejectedValueOnce(new Error("service unavailable"))
    chooseFile()
    await waitFor(() => expect(mockHandled).toBeDefined())
    await expect(mockHandled).resolves.toBeUndefined()
    expect(mockNotify).toHaveBeenCalledTimes(1)
    expect(mockNotify).toHaveBeenCalledWith("electionScreen.error.fileError", {type: "error"})
    expect(mockFetch).not.toHaveBeenCalled()
})

it("uploads the file to the address it gets", async () => {
    mockGetUploadUrl.mockResolvedValueOnce({
        data: {get_upload_url: {url: "https://upload.invalid/guide", document_id: "document"}},
    })
    mockFetch.mockResolvedValueOnce({ok: true})
    chooseFile("guide.pdf")
    await waitFor(() => expect(mockHandled).toBeDefined())
    await mockHandled
    expect(mockGetUploadUrl).toHaveBeenCalledWith({
        variables: {
            name: "guide.pdf",
            media_type: "application/pdf",
            size: 4,
            election_event_id: "event",
        },
    })
    expect(mockFetch).toHaveBeenCalledWith("https://upload.invalid/guide", expect.anything())
    expect(mockNotify).toHaveBeenCalledWith("electionScreen.common.fileLoaded", {type: "success"})
})
