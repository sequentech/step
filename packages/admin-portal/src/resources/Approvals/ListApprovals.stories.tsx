// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, eventRecord, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ListApprovals} from "./ListApprovals"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    EXPORT_DOCUMENT_ID,
    graphqlCalls,
    listFilters,
    reads,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    /** Whether the election event tabs have the event yet. */
    withEvent: boolean
    onViewApproval: Mock<(id: string | number) => void>
}

let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Approvals/ListApprovals",
    component: ListApprovals,
    args: {reads: "records", empty: false, withEvent: true, onViewApproval: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason:
                "The view action is an icon button without a name, and the status chips " +
                "put white text on light colours.",
            a11y: ["button-name", "color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        await setUpApprovals(args, {
            ExportApplication: () => ({
                data: {
                    export_application: {
                        error_msg: null,
                        document_id: EXPORT_DOCUMENT_ID,
                        task_execution: {
                            id: storyId(5, 7),
                            name: "Export applications",
                            execution_status: "IN_PROGRESS",
                            created_at: FIXED_TIME,
                            start_at: FIXED_TIME,
                            end_at: null,
                            logs: null,
                            annotations: null,
                            labels: null,
                            executed_by_user: "admin",
                            tenant_id: TENANT_ID,
                            election_event_id: EVENT_ID,
                            type: "EXPORT_APPLICATION",
                        },
                    },
                },
            }),
        })
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({withEvent, onViewApproval}) => (
        <ApprovalsScreen>
            <ListApprovals
                electionEventId={EVENT_ID}
                onViewApproval={onViewApproval}
                electionEventRecord={
                    withEvent ? (eventRecord() as Sequent_Backend_Election_Event) : undefined
                }
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const applicationsRead = () => reads("getList", "sequent_backend_applications")

const row = (canvasElement: HTMLElement, applicant: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(applicant)})

export const Populated: Story = {
    parameters: {widgets: ["ApprovalsList"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const alice = await row(canvasElement, "applicant-0001")
        await expect(within(alice).getByText("Alice")).toBeVisible()
        await expect(within(alice).getByText("alice@example.test")).toBeVisible()
        // The list first asks for pending applications, then the table replaces the
        // status with the one stored on this browser, which a first visit lacks.
        await expect(await row(canvasElement, "applicant-0002")).toBeVisible()
        // ra-data-hasura compares the status column with `_ilike`.
        expect(applicationsRead()[0].args[1]).toMatchObject({
            filter: {"election_event_id": EVENT_ID, "status@_ilike": "pending"},
            sort: {field: "created_at", order: "DESC"},
        })
        expect(listFilters("sequent_backend_applications").at(-1)).toEqual({
            election_event_id: EVENT_ID,
        })
        expect(canvas.queryByRole("progressbar")).toBeNull()
    },
}

export const StoredStatusFilter: Story = {
    args: {storedStatus: "accepted"},
    parameters: {widgets: ["ApprovalsList"]},
    play: async ({canvasElement}) => {
        const bob = await row(canvasElement, "applicant-0002")
        await expect(within(bob).getByText("admin")).toBeVisible()
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /applicant-0001/})).toBeNull()
        )
        expect(listFilters("sequent_backend_applications").at(-1)).toEqual({
            "election_event_id": EVENT_ID,
            "status@_ilike": "accepted",
        })
    },
}

export const WithoutEvent: Story = {
    args: {withEvent: false},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the event arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
        expect(applicationsRead()).toEqual([])
    },
}

export const ViewAnApplication: Story = {
    play: async ({canvasElement, args}) => {
        const alice = await row(canvasElement, "applicant-0001")
        await userEvent.click(within(alice).getByRole("button"))
        expect(args.onViewApproval).toHaveBeenCalledWith(APPLICATION_ID)
    },
}

export const ExportTheApplications: Story = {
    play: async ({canvasElement}) => {
        await row(canvasElement, "applicant-0001")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
        const dialogElement = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialogElement).toBeVisible())
        const dialog = within(dialogElement)
        await expect(dialog.getByText("Export applications")).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "export-applications.csv", href: documentUrl(EXPORT_DOCUMENT_ID)},
            ])
        )
        expect(graphqlCalls().find(({name}) => name === "ExportApplication")?.variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
        })
        const message = await within(document.body).findByText(
            "Applications export finished successfully"
        )
        await waitFor(() => expect(message).toBeVisible())
        await waitFor(() => expect(dialogElement).not.toBeInTheDocument())
    },
}
