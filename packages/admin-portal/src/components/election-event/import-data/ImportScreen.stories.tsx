// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import {ImportScreen} from "./ImportScreen"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"

const DOCUMENT_ID = "33333333-3333-4333-8333-333333333333"
const UPLOAD_URL = "https://admin-story.invalid/upload/event"
const CHECKSUM = "01".repeat(32)
const FILE_CONTENT = '{"elections":[]}'
let boundary: ReturnType<typeof graphqlBoundary>
let upload: ReturnType<typeof fn<typeof fetch>>

type Props = React.ComponentProps<typeof ImportScreen> & {
    outcome: "success" | "url-error" | "missing-url" | "http-error" | "network-error"
}

const meta = {
    title: "Admin/Event import",
    component: ImportScreen,
    args: {
        doImport: fn(async () => {}),
        uploadCallback: fn(async () => {}),
        doCancel: fn(),
        errors: null,
        outcome: "success",
    },
    beforeEach: ({args}) => {
        boundary = graphqlBoundary({
            GetUploadUrl: () => {
                if (args.outcome === "url-error") throw new Error("Upload URL service unavailable")
                return {
                    data: {
                        get_upload_url: {
                            url: args.outcome === "missing-url" ? null : UPLOAD_URL,
                            document_id: DOCUMENT_ID,
                        },
                    },
                }
            },
        })
        upload = fn<typeof fetch>(async (url) => {
            if (url !== UPLOAD_URL) throw new Error(`Unexpected upload URL: ${String(url)}`)
            if (args.outcome === "network-error") throw new TypeError("Upload connection failed")
            return new Response(null, {status: args.outcome === "http-error" ? 503 : 200})
        })
        const fetchMock = spyOn(window, "fetch").mockImplementation(upload)
        return () => fetchMock.mockRestore()
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <ImportScreen {...args} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

async function selectFile(canvasElement: HTMLElement, filename = "event.json") {
    const input = canvasElement.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import file input is missing")
    const file = new File([FILE_CONTENT], filename, {type: "application/json"})
    await userEvent.upload(input, file)
    return file
}

async function expectUploaded(file: File) {
    await waitFor(() => expect(upload).toHaveBeenCalledTimes(1))
    expect(upload).toHaveBeenCalledWith(UPLOAD_URL, {
        method: "PUT",
        headers: {"Content-Type": "application/json"},
        body: file,
    })
    expect(boundary.calls).toEqual([
        {
            name: "GetUploadUrl",
            variables: {
                name: file.name,
                media_type: file.name.endsWith(".ezip") ? "application/ezip" : "application/json",
                size: 16,
                is_public: false,
            },
            headers: {},
        },
    ])
    expect(boundary.unexpected).toEqual([])
}

export const PlainFileWithIntegrityCheck: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Import"})).toBeDisabled()
        await userEvent.type(
            canvas.getByRole("textbox", {name: "Integrity Check (SHA-256)"}),
            CHECKSUM
        )
        const file = await selectFile(canvasElement)
        await expectUploaded(file)
        await waitFor(() =>
            expect(args.uploadCallback).toHaveBeenCalledWith(DOCUMENT_ID, "", CHECKSUM)
        )
        await userEvent.click(canvas.getByRole("button", {name: "Import"}))
        await expect(args.doImport).toHaveBeenCalledWith(DOCUMENT_ID, CHECKSUM, "")
    },
}

export const ConfirmMissingIntegrityCheck: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await selectFile(canvasElement)
        await waitFor(() => expect(args.uploadCallback).toHaveBeenCalledTimes(1))
        await userEvent.click(canvas.getByRole("button", {name: "Import"}))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await waitFor(() =>
            expect(dialog.getByText("Import Without Integrity Check?")).toBeVisible()
        )
        await expect(args.doImport).not.toHaveBeenCalled()
        await userEvent.click(dialog.getByRole("button", {name: "Go Back"}))
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        await expect(args.doImport).not.toHaveBeenCalled()
        await userEvent.click(canvas.getByRole("button", {name: "Import"}))
        await userEvent.click(
            within(await within(document.body).findByRole("dialog")).getByRole("button", {
                name: "Yes, Import without Integrity Check",
            })
        )
        await expect(args.doImport).toHaveBeenCalledWith(DOCUMENT_ID, "", "")
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
    },
}

export const EncryptedFileRequiresPassword: Story = {
    play: async ({canvasElement, args}) => {
        const file = await selectFile(canvasElement, "event.ezip")
        const dialogElement = await within(document.body).findByRole("dialog")
        const dialog = within(dialogElement)
        await waitFor(() => expect(dialog.getByText("Decryption Password")).toBeVisible())
        await expect(upload).not.toHaveBeenCalled()
        await expect(boundary.calls).toEqual([])
        const password = dialog.getByLabelText("Password", {selector: "input"})
        await userEvent.type(password, "correct horse battery")
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        await expectUploaded(file)
        await waitFor(() =>
            expect(args.uploadCallback).toHaveBeenCalledWith(
                DOCUMENT_ID,
                "correct horse battery",
                ""
            )
        )
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
    },
}

async function assertUploadFailure(canvasElement: HTMLElement, args: Props) {
    await selectFile(canvasElement)
    await within(document.body).findByText("Error uploading file")
    await expect(args.uploadCallback).not.toHaveBeenCalled()
    await expect(args.doImport).not.toHaveBeenCalled()
    await expect(within(canvasElement).getByRole("button", {name: "Import"})).toBeDisabled()
    await expect(
        within(canvasElement).getByRole("textbox", {name: "Integrity Check (SHA-256)"})
    ).toBeEnabled()
    expect(boundary.calls).toHaveLength(1)
    expect(boundary.unexpected).toEqual([])
}

export const UploadUrlFailure: Story = {
    args: {outcome: "url-error"},
    play: async ({canvasElement, args}) => {
        await assertUploadFailure(canvasElement, args)
        expect(upload).not.toHaveBeenCalled()
    },
}
export const UploadConnectionFailure: Story = {
    args: {outcome: "network-error"},
    play: async ({canvasElement, args}) => {
        await assertUploadFailure(canvasElement, args)
        expect(upload).toHaveBeenCalledTimes(1)
    },
}
export const UploadHttpFailure: Story = {
    args: {outcome: "http-error"},
    play: async ({canvasElement, args}) => {
        await assertUploadFailure(canvasElement, args)
        expect(upload).toHaveBeenCalledTimes(1)
    },
}
export const MissingUploadUrl: Story = {
    args: {outcome: "missing-url"},
    play: async ({canvasElement, args}) => {
        await assertUploadFailure(canvasElement, args)
        expect(upload).not.toHaveBeenCalled()
    },
}
export const CancelBeforeUploading: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Cancel"}))
        await expect(args.doCancel).toHaveBeenCalledTimes(1)
        expect(boundary.calls).toEqual([])
        expect(upload).not.toHaveBeenCalled()
    },
}

export const CancelEncryptedFile: Story = {
    play: async ({canvasElement, args}) => {
        await selectFile(canvasElement, "event.ezip")
        const dialogElement = await within(document.body).findByRole("dialog")
        const dialog = within(dialogElement)
        const password = dialog.getByLabelText("Password", {selector: "input"})
        await waitFor(() => expect(password).toBeVisible())
        await userEvent.click(password)
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
        )
        expect(boundary.calls).toEqual([])
        expect(upload).not.toHaveBeenCalled()
        expect(args.uploadCallback).not.toHaveBeenCalled()
        expect(args.doImport).not.toHaveBeenCalled()
        await expect(within(canvasElement).getByRole("button", {name: "Import"})).toBeDisabled()
    },
}
