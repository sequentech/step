// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/__stories__/downloads"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {SettingsPreviews} from "./SettingsPreviews"
import {
    PREVIEW_DOCUMENT_ID,
    PREVIEW_RESOURCE,
    previewRecords,
    settingsTab,
} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the previews does. */
    reads: ReadState
    /** Whether there are previews. */
    populated: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

const Tab = settingsTab(SettingsPreviews)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Settings/SettingsPreviews",
    component: SettingsPreviews,
    args: {reads: "records", populated: true, roles: [IPermissions.PREVIEW_READ]},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled and each row's download action is an icon button without an accessible name.",
            a11y: ["aria-prohibited-attr", "button-name", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[PREVIEW_RESOURCE]: args.populated ? previewRecords() : []},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(
            documentHandlers({[PREVIEW_DOCUMENT_ID]: {name: "council-preview.json"}}),
            {schema: true}
        )
        await graphql.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <Tab />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const PREVIEW_URL = "https://voting.admin-story.invalid/preview/council"

const previewRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: /admin/})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const row = within(await previewRow(canvasElement))
        await expect(row.getByRole("link", {name: PREVIEW_URL})).toHaveAttribute(
            "href",
            PREVIEW_URL
        )
        await expect(
            within(canvasElement).getByText(i18n.t("settings.previewScreen.table.title"))
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", PREVIEW_RESOURCE],
        ])
        expect(graphql.calls).toEqual([])
    },
}

export const DownloadAPreview: Story = {
    play: async ({canvasElement}) => {
        const row = within(await previewRow(canvasElement))
        await userEvent.click(row.getAllByRole("button").at(-1) as HTMLElement)
        // Without a file name of its own, the preview is saved under the document's.
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "council-preview.json", href: documentUrl(PREVIEW_DOCUMENT_ID)},
            ])
        )
        expect(graphql.calls.map(({name, variables}) => [name, variables])).toEqual(
            expect.arrayContaining([
                ["GetDocument", {id: PREVIEW_DOCUMENT_ID, tenantId: TENANT_ID}],
                ["FetchDocument", {documentId: PREVIEW_DOCUMENT_ID}],
            ])
        )
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(i18n.t("settings.previewScreen.noContent"))
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const WithoutPreviewPermission: Story = {
    args: {roles: []},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(i18n.t("settings.previewScreen.noContent"))
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByRole("row", {name: /admin/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /admin/})).toBeNull()
    },
}
