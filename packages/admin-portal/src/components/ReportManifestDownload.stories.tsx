// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type StoryDocument,
} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import {ReportManifestDownload} from "./ReportManifestDownload"

const REPORT_ID = storyId(9, 21)
const MANIFEST_ID = storyId(9, 22)

interface Scenario {
    /** The generated report's document. */
    report: StoryDocument
    disabled: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let saved: ReturnType<typeof recordDownloads>

const stamped: StoryDocument = {
    name: "activity-logs.pdf",
    annotations: {
        report_manifest_file: {document_id: MANIFEST_ID, sha256: "ab".repeat(32)},
    },
}

const meta = {
    title: "Admin/Components/ReportManifestDownload",
    component: ReportManifestDownload,
    args: {report: stamped, disabled: false},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            documentHandlers({
                [REPORT_ID]: args.report,
                [MANIFEST_ID]: {name: "report-manifest.json"},
            }),
            {schema: true}
        )
        await graphql.ready
        saved = recordDownloads()
        return saved.restore
    },
    render: ({disabled}) => (
        <AdminStoryProvider boundary={graphql} role={EStoryPermissions.ADMIN}>
            <ReportManifestDownload
                documentId={REPORT_ID}
                electionEventId={STORY_IDS.event}
                disabled={disabled}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const DownloadsTheHashManifest: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Hash manifest"}))
        await waitFor(() =>
            expect(saved.downloads).toEqual([
                {name: "report-manifest.json", href: documentUrl(MANIFEST_ID)},
            ])
        )
        expect(graphql.calls[0]).toEqual({
            name: "GetDocument",
            variables: {id: REPORT_ID, tenantId: TENANT_ID},
            headers: {},
        })
        expect(graphql.calls.filter(({name}) => name === "FetchDocument")[0]).toEqual({
            name: "FetchDocument",
            variables: {electionEventId: STORY_IDS.event, documentId: MANIFEST_ID},
            headers: {},
        })
        await waitFor(() =>
            expect(canvas.getByRole("button", {name: "Hash manifest"})).toBeEnabled()
        )
    },
}

export const ReportWithoutAHashManifest: Story = {
    args: {report: {name: "activity-logs.pdf"}},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(graphql.calls.map(({name}) => name)).toEqual(["GetDocument"]))
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(saved.downloads).toEqual([])
    },
}

export const ReportNotGeneratedYet: Story = {
    args: {disabled: true},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}
