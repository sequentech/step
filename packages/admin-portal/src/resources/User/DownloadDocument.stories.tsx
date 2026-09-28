// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type StoryDocument,
} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {DownloadDocument} from "./DownloadDocument"

const DOCUMENT_ID = storyId(9, 1)
const PASSWORD = "synthetic-report-password"

interface Scenario {
    /** The stored document, or none while it is still being generated. */
    document: StoryDocument | null
    fileName: string | null
    showReportPasswordDialog: boolean
    withProgress: boolean
    role: EStoryPermissions
    onDownload: () => void
    onSuccess: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let saved: ReturnType<typeof recordDownloads>

const encryptedReport: StoryDocument = {
    name: "voters-report.epdf",
    annotations: {access: {password_secret_id: "secret-1"}},
}

const meta = {
    title: "Admin/User/DownloadDocument",
    component: DownloadDocument,
    args: {
        document: {name: "export-voters.csv"},
        fileName: "voters.csv",
        showReportPasswordDialog: false,
        withProgress: true,
        role: EStoryPermissions.ADMIN,
        onDownload: fn(),
        onSuccess: fn(),
    },
    argTypes: {role: {control: "select", options: Object.values(EStoryPermissions)}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                ...documentHandlers(args.document ? {[DOCUMENT_ID]: args.document} : {}),
                GetDocumentPassword: () => ({
                    data: {get_document_password: {password: PASSWORD}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
        saved = recordDownloads()
        return saved.restore
    },
    render: ({document: _document, role, ...args}) => (
        <AdminStoryProvider boundary={graphql} role={role}>
            <DownloadDocument
                documentId={DOCUMENT_ID}
                electionEventId={STORY_IDS.event}
                {...args}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const lookups = [
    {name: "GetDocument", variables: {id: DOCUMENT_ID, tenantId: TENANT_ID}, headers: {}},
    {
        name: "FetchDocument",
        variables: {electionEventId: STORY_IDS.event, documentId: DOCUMENT_ID},
        headers: {},
    },
]

export const DownloadWithTheGivenName: Story = {
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(saved.downloads).toEqual([{name: "voters.csv", href: documentUrl(DOCUMENT_ID)}])
        )
        await waitFor(() => expect(args.onDownload).toHaveBeenCalledTimes(1))
        expect(args.onSuccess).toHaveBeenCalledTimes(1)
        expect(graphql.calls.slice(0, 2)).toEqual(expect.arrayContaining(lookups))
        await waitFor(() => expect(within(canvasElement).queryByRole("progressbar")).toBeNull())
    },
}

export const DownloadWithTheStoredName: Story = {
    args: {fileName: null},
    play: async ({args}) => {
        await waitFor(() =>
            expect(saved.downloads).toEqual([
                {name: "export-voters.csv", href: documentUrl(DOCUMENT_ID)},
            ])
        )
        await waitFor(() => expect(args.onDownload).toHaveBeenCalledTimes(1))
    },
}

export const WaitingForTheDocument: Story = {
    args: {document: null},
    parameters: {
        expectedFailure: {
            reason: "The download spinner is a progressbar without an accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name).sort()).toEqual([
                "FetchDocument",
                "GetDocument",
            ])
        )
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(saved.downloads).toEqual([])
        expect(args.onDownload).not.toHaveBeenCalled()
    },
}

export const WithoutProgress: Story = {
    args: {document: null, withProgress: false},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(graphql.calls).toHaveLength(2))
        expect(within(canvasElement).queryByRole("progressbar")).toBeNull()
        expect(saved.downloads).toEqual([])
    },
}

export const EncryptedReportShowsItsPassword: Story = {
    args: {document: encryptedReport, fileName: "report.pdf", showReportPasswordDialog: true},
    play: async ({args}) => {
        const body = within(document.body)
        // The stored name keeps the .epdf extension that decryption needs.
        await waitFor(() =>
            expect(saved.downloads).toEqual([
                {name: "voters-report.epdf", href: documentUrl(DOCUMENT_ID)},
            ])
        )
        const dialog = within(await body.findByRole("dialog"))
        await expect(await dialog.findByDisplayValue(PASSWORD)).toBeVisible()
        expect(graphql.calls.find(({name}) => name === "GetDocumentPassword")).toEqual({
            name: "GetDocumentPassword",
            variables: {documentId: DOCUMENT_ID},
            headers: {"x-hasura-role": "document-password-read"},
        })
        expect(args.onDownload).not.toHaveBeenCalled()
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        await waitFor(() => expect(args.onDownload).toHaveBeenCalledTimes(1))
        await waitFor(() => expect(body.queryByRole("dialog")).toBeNull())
    },
}

export const EncryptedReportWithoutPasswordPermission: Story = {
    args: {
        document: encryptedReport,
        showReportPasswordDialog: true,
        role: EStoryPermissions.ADMIN_LIGHT,
    },
    play: async ({args}) => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(
            await dialog.findByText("The PDF password could not be retrieved")
        ).toBeVisible()
        await expect(dialog.getByText("How to decrypt the file")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).not.toContain("GetDocumentPassword")
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        await waitFor(() => expect(args.onDownload).toHaveBeenCalledTimes(1))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const LegacyEncryptedReport: Story = {
    args: {document: {name: "old-report.epdf"}, showReportPasswordDialog: true},
    parameters: {
        expectedFailure: {
            reason: "The decryption command is shown in a read-only text field without a label.",
            a11y: ["label"],
        },
    },
    play: async () => {
        const dialog = within(await within(document.body).findByRole("dialog"))
        // Without a saved password only the decryption instructions are shown.
        await expect(await dialog.findByText("How to decrypt the file")).toBeVisible()
        expect(dialog.queryByText("The PDF password could not be retrieved")).toBeNull()
        expect(graphql.calls.map(({name}) => name)).not.toContain("GetDocumentPassword")
    },
}
