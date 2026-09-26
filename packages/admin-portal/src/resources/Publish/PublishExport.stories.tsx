// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/__stories__/downloads"
import {STORY_IDS, electionRecord, eventRecord, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import PublishExport from "./PublishExport"

interface Scenario {
    /** Exports from an election's publish tab instead of the event's. */
    election: boolean
    /** Whether the export service rejects the request. */
    exportFails: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]

const PUBLICATION_ID = storyId(8, 2)
const DOCUMENT_ID = storyId(8, 4)
const TASK_ID = storyId(8, 5)

const meta = {
    title: "Admin/Publish/PublishExport",
    component: PublishExport,
    args: {election: false, exportFails: false},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                ExportBallotPublication: () =>
                    args.exportFails
                        ? {errors: [new GraphQLError("Synthetic export rejected")]}
                        : {
                              data: {
                                  export_ballot_publication: {
                                      document_id: DOCUMENT_ID,
                                      task_execution: startedTask(
                                          TASK_ID,
                                          "EXPORT_BALLOT_PUBLICATION"
                                      ),
                                  },
                              },
                          },
                ...taskHandler("SUCCESS", "EXPORT_BALLOT_PUBLICATION"),
                ...documentHandlers({[DOCUMENT_ID]: {name: "ballot-publication.json"}}),
            },
            {schema: true}
        )
        await graphql.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({election}) => (
        <AdminStoryProvider boundary={graphql}>
            <WidgetsContextProvider>
                <RecordContextProvider value={election ? electionRecord() : eventRecord()}>
                    <PublishExport ballotPublicationId={PUBLICATION_ID} />
                </RecordContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function openExport(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return dialog
}

async function exportPublication(canvasElement: HTMLElement) {
    const dialog = await openExport(canvasElement)
    await userEvent.click(within(dialog).getByRole("button", {name: "Export"}))
    return dialog
}

export const Closed: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("button", {name: "Export"})).toBeEnabled()
        expect(within(document.body).queryByRole("dialog")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const ExportAnEventPublication: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        const dialog = await exportPublication(canvasElement)
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "ballot-publication-export.json", href: documentUrl(DOCUMENT_ID)},
            ])
        )
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(graphql.calls[0]).toEqual({
            name: "ExportBallotPublication",
            variables: {
                tenantId: TENANT_ID,
                electionEventId: EVENT_ID,
                electionId: null,
                ballotPublicationId: PUBLICATION_ID,
            },
            headers: expect.objectContaining({"x-hasura-role": "publish-write"}),
        })
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ExportAnElectionPublication: Story = {
    args: {election: true},
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        const dialog = await exportPublication(canvasElement)
        await waitFor(() => expect(downloads).toHaveLength(1))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(graphql.calls[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            electionId: STORY_IDS.election,
            ballotPublicationId: PUBLICATION_ID,
        })
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ExportFailure: Story = {
    args: {exportFails: true},
    parameters: taskWidgetDefects("FAILED"),
    play: async ({canvasElement}) => {
        const dialog = await exportPublication(canvasElement)
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        // The dialog closes, so the export can be tried again.
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        const retry = await exportPublication(canvasElement)
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual([
                "ExportBallotPublication",
                "ExportBallotPublication",
            ])
        )
        await waitFor(() => expect(retry).not.toBeInTheDocument())
        expect(downloads).toEqual([])
    },
}

export const CancelTheExport: Story = {
    play: async ({canvasElement}) => {
        const dialog = await openExport(canvasElement)
        await userEvent.click(within(dialog).getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(graphql.calls).toEqual([])
        expect(downloads).toEqual([])
    },
}
