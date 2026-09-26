// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EReportType} from "@/types/reports"
import {EditReportForm, EReportEncryption} from "./EditReportForm"
import {
    ELECTIONS,
    REPORTS,
    REPORT_IDS,
    REPORT_ROLES,
    TEMPLATES,
    scheduleDefects,
} from "./__stories__/ReportsFixture"

interface Scenario {
    /** The report being edited; none creates one. */
    reportId: string | null
    /** Whether the report service rejects the save. */
    saveFails: boolean
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const CREATED_ID = storyId(1, 4)
const PASSWORD = "Report-synthetic-9"

const meta = {
    title: "Admin/Reports/EditReportForm",
    component: EditReportForm,
    args: {reportId: null, saveFails: false, close: fn()},
    argTypes: {close: {table: {disable: true}}},
    parameters: {
        widgets: ["FormContent", "ReportTypeInput", "PermissionLabelsInput", "PasswordComponent"],
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({
            sequent_backend_report: REPORTS,
            sequent_backend_template: TEMPLATES,
            sequent_backend_election: ELECTIONS,
        })
        const rejected = {errors: [new GraphQLError("Synthetic report rejected")]}
        graphql = graphqlBoundary(
            {
                InsertReport: ({variables}) =>
                    args.saveFails
                        ? rejected
                        : {
                              data: {
                                  insert_sequent_backend_report: {
                                      affected_rows: 1,
                                      returning: [{...variables.object, id: CREATED_ID}],
                                  },
                              },
                          },
                UpdateReport: ({variables}) =>
                    args.saveFails
                        ? rejected
                        : {data: {update_sequent_backend_report_by_pk: {id: variables.id}}},
                EncryptReport: () => ({
                    data: {encrypt_report: {document_id: storyId(1, 5), error_msg: null}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({reportId, close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={REPORT_ROLES}>
            <ResourceContextProvider value="sequent_backend_election_event">
                <EditReportForm
                    close={close}
                    electionEventId={EVENT_ID}
                    tenantId={TENANT_ID}
                    isEditReport={!!reportId}
                    reportId={reportId}
                />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const notice = (text: string) =>
    waitFor(() => expect(within(document.body).getByText(text)).toBeVisible())

async function chooseReportType(canvasElement: HTMLElement, name: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: "Type"}))
    await userEvent.click(await within(document.body).findByRole("option", {name}))
}

async function setPassword(canvasElement: HTMLElement, password: string, repeat: string) {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: /Encryption Policy/}))
    await userEvent.click(
        await within(document.body).findByRole("option", {name: "Configured Password"})
    )
    const dialog = await within(document.body).findByRole("dialog", {name: "Password"})
    const [first, second] = Array.from(dialog.querySelectorAll<HTMLInputElement>("input"))
    await userEvent.type(first, password)
    await userEvent.type(second, repeat)
    return dialog
}

const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))

export const CreateAReport: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Create Report")).toBeVisible()
        await chooseReportType(canvasElement, "Activity Logs")
        await save(canvasElement)
        await notice("Report created successfully")
        expect(graphql.calls).toEqual([
            {
                name: "InsertReport",
                variables: {
                    object: expect.objectContaining({
                        report_type: EReportType.ACTIVITY_LOGS,
                        encryption_policy: EReportEncryption.UNENCRYPTED,
                        tenant_id: TENANT_ID,
                        election_event_id: EVENT_ID,
                        cron_config: null,
                    }),
                },
                headers: {},
            },
        ])
        expect(args.close).toHaveBeenCalledOnce()
    },
}

export const CreateAnEncryptedReport: Story = {
    play: async ({canvasElement, args}) => {
        await chooseReportType(canvasElement, "Activity Logs")
        const dialog = await setPassword(canvasElement, PASSWORD, PASSWORD)
        await userEvent.click(within(dialog).getByRole("button", {name: "Save Password"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        await save(canvasElement)
        await notice("Report created successfully")
        await waitFor(() => expect(args.close).toHaveBeenCalledOnce())
        expect(graphql.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "InsertReport",
                variables: {
                    object: expect.objectContaining({
                        encryption_policy: EReportEncryption.CONFIGURED_PASSWORD,
                    }),
                },
            },
            {
                name: "EncryptReport",
                variables: {reportId: CREATED_ID, electionEventId: EVENT_ID, password: PASSWORD},
            },
        ])
        // The password is sent to the encryption service only, never stored with the report.
        const [insert] = graphql.calls
        expect(JSON.stringify(insert.variables)).not.toContain(PASSWORD)
    },
}

export const PasswordsDoNotMatch: Story = {
    play: async ({canvasElement}) => {
        const dialog = await setPassword(canvasElement, PASSWORD, "Different-9")
        await expect(within(dialog).getByRole("button", {name: "Save Password"})).toBeDisabled()
        await userEvent.keyboard("{Escape}")
        await notice(
            "Password and confirm password do not match. Please ensure both fields contain the same password."
        )
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(graphql.calls).toEqual([])
    },
}

export const EditAReport: Story = {
    args: {reportId: REPORT_IDS.participation},
    parameters: {
        widgets: [
            "FormContent",
            "ReportTypeInput",
            "PermissionLabelsInput",
            "EmailRecipientsInput",
            "PasswordComponent",
        ],
        expectedFailure: scheduleDefects,
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(canvas.getByText("Edit Report")).toBeVisible())
        await waitFor(() =>
            expect(canvas.getByRole("combobox", {name: "Type"})).toHaveValue("Participation Report")
        )
        await expect(await canvas.findByText("observer@example.org")).toBeVisible()
        await expect(canvas.getByText("north")).toBeVisible()
        // Saving is enabled once something changes.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        await userEvent.type(
            canvas.getByRole("combobox", {name: "Email Recipients"}),
            "auditor@example.org{Enter}"
        )
        await save(canvasElement)
        await notice("Report updated successfully")
        expect(graphql.calls).toEqual([
            expect.objectContaining({
                name: "UpdateReport",
                variables: {
                    id: REPORT_IDS.participation,
                    set: expect.objectContaining({
                        report_type: EReportType.PARTICIPATION_REPORT,
                        template_alias: "participation-summary",
                        cron_config: expect.objectContaining({
                            is_active: true,
                            email_recipients: ["observer@example.org", "auditor@example.org"],
                        }),
                    }),
                },
            }),
        ])
        expect(args.close).toHaveBeenCalledOnce()
    },
}

export const SaveFailure: Story = {
    args: {saveFails: true},
    play: async ({canvasElement, args}) => {
        await chooseReportType(canvasElement, "Activity Logs")
        await save(canvasElement)
        await notice("Error submitting Report")
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const RepeatableReportNeedsASchedule: Story = {
    parameters: {
        widgets: ["FormContent", "ReportTypeInput", "EmailRecipientsInput", "PasswordComponent"],
        expectedFailure: scheduleDefects,
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await chooseReportType(canvasElement, "Participation Report")
        await userEvent.click(await canvas.findByLabelText("Repeatable"))
        await expect(await canvas.findByRole("combobox", {name: "Email Recipients"})).toBeVisible()
        await save(canvasElement)
        await notice("Please configure a cron schedule before saving")
        expect(graphql.calls).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}
