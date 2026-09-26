// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider, type DataProvider, type Identifier} from "react-admin"
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
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    startedTask,
    taskHandler,
    taskWidgetDefects,
} from "@/components/tally/__stories__/DownloadFixture"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EGenerateReportMode} from "@/types/reports"
import {EReportEncryption} from "./EditReportForm"
import ListReports from "./ListReports"
import {
    ELECTIONS,
    REPORTS,
    REPORT_IDS,
    REPORT_ROLES,
    TEMPLATES,
    scheduleDefects,
} from "./__stories__/ReportsFixture"

interface Scenario {
    roles: string[]
    /** Whether the event has any report. */
    reports: boolean
    electionReads: ReadState
    /** Whether the report service rejects a generation. */
    generationFails: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]

const DOCUMENT_IDS = {plain: storyId(1, 6), encrypted: storyId(1, 7)}
const TASK_ID = storyId(1, 8)
const PASSWORD = "Report-synthetic-9"

const encrypted = (variables: Record<string, unknown>) =>
    variables.reportId === REPORT_IDS.participation

/** Axe defects of the report list: its row checkboxes have no usable name. */
const listDefects = {
    reason:
        "The row selection checkboxes carry an aria-label on a span with no role and " +
        "their inputs have no label.",
    a11y: ["aria-prohibited-attr", "label"],
}

/** The list's defects, and those of the task widget a generation opens. */
const withTaskWidget = (status: "SUCCESS" | "FAILED") => {
    const {expectedFailure} = taskWidgetDefects(status)
    return {
        expectedFailure: {
            reason: `${listDefects.reason} ${expectedFailure.reason}`,
            a11y: [...listDefects.a11y, ...expectedFailure.a11y],
        },
    }
}

/** The report drawer is modal and hides the list; it has no accessible name. */
const drawerDefects = (extra: string[] = [], reason = "") => ({
    expectedFailure: {
        reason: `The report drawer has no accessible name.${reason}`,
        a11y: ["aria-dialog-name", ...extra],
    },
})

/**
 * Deleting a report refreshes every query, including the one for the report
 * just deleted, which Hasura answers without a record.
 */
function withDeletedReports(provider: DataProvider): DataProvider {
    const deleted = new Set<Identifier>()
    return {
        ...provider,
        delete: async (resource, params) => {
            const result = await provider.delete(resource, params)
            deleted.add(params.id)
            return result
        },
        getOne: (resource, params) =>
            deleted.has(params.id)
                ? Promise.reject(new Error(`${resource} ${params.id} not found`))
                : provider.getOne(resource, params),
    }
}

const meta = {
    title: "Admin/Reports/ListReports",
    component: ListReports,
    args: {roles: REPORT_ROLES, reports: true, electionReads: "records", generationFails: false},
    argTypes: {electionReads: {control: "inline-radio", options: ["records", "loading"]}},
    parameters: {
        widgets: ["ActionsPopUp"],
        expectedFailure: listDefects,
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                sequent_backend_report: args.reports ? REPORTS : [],
                sequent_backend_template: TEMPLATES,
                sequent_backend_election: ELECTIONS,
            },
            {reads: {sequent_backend_election: args.electionReads}}
        )
        graphql = graphqlBoundary(
            {
                GenerateReport: ({variables}) =>
                    args.generationFails
                        ? {errors: [new GraphQLError("Synthetic report generation rejected")]}
                        : {
                              data: {
                                  generate_report: {
                                      document_id: encrypted(variables)
                                          ? DOCUMENT_IDS.encrypted
                                          : DOCUMENT_IDS.plain,
                                      encryption_policy: encrypted(variables)
                                          ? EReportEncryption.CONFIGURED_PASSWORD
                                          : EReportEncryption.UNENCRYPTED,
                                      task_execution: startedTask(TASK_ID, "GENERATE_REPORT"),
                                  },
                              },
                          },
                GetDocumentPassword: () => ({
                    data: {get_document_password: {password: PASSWORD}},
                }),
                ...taskHandler("SUCCESS", "GENERATE_REPORT"),
                ...documentHandlers({
                    [DOCUMENT_IDS.plain]: {name: "activity-logs.pdf"},
                    [DOCUMENT_IDS.encrypted]: {
                        name: "participation.epdf",
                        annotations: {access: {password_secret_id: storyId(1, 9)}},
                    },
                }),
            },
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
            dataProvider={withDeletedReports(data.provider)}
            roles={roles}
            // The report form's own Apollo client is created for a signed-in user.
            auth={{isAuthenticated: true, getAccessToken: () => "story-access-token"}}
        >
            <WidgetsContextProvider>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <ListReports electionEventId={EVENT_ID} />
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const reportRow = (canvasElement: HTMLElement, type: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(` ${type} `)})

/** The row's cells after its selection checkbox. */
const cells = (row: HTMLElement) =>
    within(row)
        .getAllByRole("cell")
        .slice(1)
        .map((cell) => cell.textContent)

async function openActions(canvasElement: HTMLElement, type: string) {
    const row = await reportRow(canvasElement, type)
    await userEvent.click(within(row).getByRole("button", {name: "Actions"}))
    const menu = await within(document.body).findByRole("menu")
    await waitFor(() => expect(menu).toBeVisible())
    return menu
}

async function chooseAction(canvasElement: HTMLElement, type: string, action: string) {
    const menu = await openActions(canvasElement, type)
    await userEvent.click(within(menu).getByRole("menuitem", {name: action}))
    await waitFor(() => expect(menu).not.toBeVisible())
}

const menuItems = (menu: HTMLElement) =>
    within(menu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Reports")).toBeVisible()
        const participation = await reportRow(canvasElement, "Participation Report")
        await within(participation).findByText("Participation summary")
        expect(cells(participation).slice(0, 3)).toEqual([
            "Participation Report",
            "Participation summary",
            "Council",
        ])
        const activity = await reportRow(canvasElement, "Activity Logs")
        expect(cells(activity).slice(0, 3)).toEqual(["Activity Logs", "-", "-"])
        expect(canvas.getAllByRole("row")).toHaveLength(4)
        expect(data.writes).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}

export const ActionsFollowTheReportType: Story = {
    play: async ({canvasElement}) => {
        let menu = await openActions(canvasElement, "Activity Logs")
        expect(menuItems(menu)).toEqual(["Edit", "Delete", "Generate", "Preview"])
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(menu).not.toBeVisible())
        // An initialization report is only previewed.
        menu = await openActions(canvasElement, "Initialization Report")
        expect(menuItems(menu)).toEqual(["Edit", "Delete", "Preview"])
        await userEvent.keyboard("{Escape}")
        await waitFor(() => expect(menu).not.toBeVisible())
    },
}

export const EditAReport: Story = {
    parameters: {
        widgets: ["ActionsPopUp", "EditReportForm"],
        ...drawerDefects(scheduleDefects.a11y, ` ${scheduleDefects.reason}`),
    },
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Participation Report", "Edit")
        const drawer = within(document.body).getByRole("presentation")
        await waitFor(() => expect(within(drawer).getByText("Edit Report")).toBeVisible())
        await waitFor(() =>
            expect(within(drawer).getByRole("combobox", {name: "Type"})).toHaveValue(
                "Participation Report"
            )
        )
    },
}

export const CreateAReport: Story = {
    parameters: {widgets: ["ActionsPopUp", "EditReportForm"], ...drawerDefects()},
    play: async ({canvasElement}) => {
        await reportRow(canvasElement, "Activity Logs")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Add"}))
        const drawer = await within(document.body).findByRole("presentation")
        await waitFor(() => expect(within(drawer).getByText("Create Report")).toBeVisible())
        await expect(within(drawer).getByRole("combobox", {name: "Type"})).toHaveValue("")
    },
}

export const DeleteAReport: Story = {
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Initialization Report", "Delete")
        const dialog = await within(document.body).findByRole("dialog")
        await expect(
            within(dialog).getByText("Are you sure you want delete this Report?")
        ).toBeVisible()
        await userEvent.click(within(dialog).getByRole("button", {name: "Delete"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(data.writes).toEqual([
            {
                method: "delete",
                resource: "sequent_backend_report",
                params: expect.objectContaining({id: REPORT_IDS.initialization}),
            },
        ])
        await waitFor(() => expect(within(canvasElement).getAllByRole("row")).toHaveLength(3))
    },
}

export const CancelADeletion: Story = {
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Initialization Report", "Delete")
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(within(dialog).getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(data.writes).toEqual([])
    },
}

export const GenerateAReport: Story = {
    parameters: withTaskWidget("SUCCESS"),
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Activity Logs", "Generate")
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "activity-logs.pdf", href: documentUrl(DOCUMENT_IDS.plain)},
            ])
        )
        expect(graphql.calls[0]).toEqual({
            name: "GenerateReport",
            variables: {
                reportId: REPORT_IDS.activityLogs,
                tenantId: TENANT_ID,
                reportMode: EGenerateReportMode.REAL,
                electionEventId: EVENT_ID,
            },
            headers: expect.objectContaining({"x-hasura-role": "report-read"}),
        })
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
        expect(within(document.body).queryByRole("dialog")).toBeNull()
    },
}

export const PreviewAnEncryptedReport: Story = {
    parameters: {
        widgets: ["ActionsPopUp", "ReportPasswordDialog"],
        // The modal password dialog hides the list and the task widget's buttons.
        expectedFailure: {
            reason:
                "The task widget's success chip has white text below 4.5 contrast; the " +
                "read-only password and decryption command fields have no label.",
            a11y: ["color-contrast", "label"],
        },
    },
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Participation Report", "Preview")
        const dialog = await within(document.body).findByRole("dialog", {name: "Password"})
        await waitFor(() => expect(within(dialog).getByDisplayValue(PASSWORD)).toBeVisible())
        expect(downloads).toEqual([
            {name: "participation.epdf", href: documentUrl(DOCUMENT_IDS.encrypted)},
        ])
        expect(graphql.calls[0].variables).toMatchObject({
            reportId: REPORT_IDS.participation,
            reportMode: EGenerateReportMode.PREVIEW,
        })
        expect(graphql.calls.map(({name}) => name)).toContain("GetDocumentPassword")
    },
}

export const GenerationFailure: Story = {
    args: {generationFails: true},
    parameters: withTaskWidget("FAILED"),
    play: async ({canvasElement}) => {
        await chooseAction(canvasElement, "Activity Logs", "Generate")
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["GenerateReport"])
        expect(downloads).toEqual([])
    },
}

export const Empty: Story = {
    args: {reports: false},
    parameters: {
        widgets: ["EditReportForm"],
        ...drawerDefects(),
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Reports yet.")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Create Report"}))
        const drawer = await within(document.body).findByRole("presentation")
        await waitFor(() => expect(within(drawer).getByText("Create Report")).toBeVisible())
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.REPORT_READ]},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await reportRow(canvasElement, "Activity Logs")
        expect(canvas.queryByRole("button", {name: "Actions"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Add"})).toBeNull()
    },
}

export const LoadingElections: Story = {
    args: {electionReads: "loading"},
    parameters: {
        widgets: [],
        expectedFailure: {
            reason: "The spinner shown while the elections load has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByRole("table")).toBeNull()
    },
}
