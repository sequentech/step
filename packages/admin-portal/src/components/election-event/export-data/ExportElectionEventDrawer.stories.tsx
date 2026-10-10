// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ETasksExecution} from "@/types/tasksExecution"
import {startedTask, taskHandler} from "@/components/tally/__stories__/DownloadFixture"
import {ExportElectionEventDrawer} from "./ExportElectionEventDrawer"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

type Outcome = "exported" | "failure"

interface Scenario {
    /** Whether the parent has opened the export dialog. */
    open: boolean
    /** What the export service answers. */
    outcome: Outcome
    setOpenExport: (open: boolean) => void
    setLoadingExport: (loading: boolean) => void
}

const TASK_ID = storyId(7, 9)
const DOCUMENT_ID = storyId(6, 9)
const PASSWORD = "synthetic-export-password"
const TYPE = ETasksExecution.EXPORT_ELECTION_EVENT

let graphql: ReturnType<typeof graphqlBoundary>

/** The event list's state: it opens the dialog and shows the export in progress. */
function Fixture({open, setOpenExport, setLoadingExport}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const [openExport, setOpen] = useState(open)
    return (
        <AdminStoryProvider boundary={graphql} role={permissions} tenant={tenant}>
            <WidgetsContextProvider>
                <ExportElectionEventDrawer
                    electionEventId={EVENT_ID}
                    openExport={openExport}
                    setOpenExport={(value) => {
                        setOpenExport(value)
                        setOpen(value)
                    }}
                    setLoadingExport={setLoadingExport}
                />
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/Export data/ExportElectionEventDrawer",
    component: ExportElectionEventDrawer,
    args: {open: true, outcome: "exported", setOpenExport: fn(), setLoadingExport: fn()},
    argTypes: {
        outcome: {control: "inline-radio", options: ["exported", "failure"]},
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                ExportElectionEvent: ({variables}) => {
                    if (args.outcome === "failure") throw new Error("Synthetic export failure")
                    const options = variables.exportConfigurations as {is_encrypted: boolean}
                    return {
                        data: {
                            export_election_event: {
                                password: options.is_encrypted ? PASSWORD : null,
                                document_id: DOCUMENT_ID,
                                task_execution: startedTask(TASK_ID, TYPE),
                            },
                        },
                    }
                },
                ...taskHandler("SUCCESS", TYPE),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const successChipDefect = {
    reason: "The task widget's success chip has white text below 4.5 contrast.",
    a11y: ["color-contrast"],
}

const NO_OPTIONS = {
    is_encrypted: false,
    encrypt_with_password: false,
    include_voters: false,
    activity_logs: false,
    bulletin_board: false,
    publications: false,
    s3_files: false,
    scheduled_events: false,
    reports: false,
    applications: false,
    tally: false,
    include_certificates: false,
}

const exportDialog = async () => {
    const element = await within(document.body).findByRole("dialog", {
        name: "Export Election Event",
    })
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const exportCall = () => graphql.calls.find(({name}) => name === "ExportElectionEvent")

export const Populated: Story = {
    play: async ({args}) => {
        const dialog = await exportDialog()
        for (const name of [
            "Encrypt with Password",
            "Include Voters",
            "Activity Logs",
            "Bulletin Board",
            "Publications",
            "S3 Files",
            "Scheduled Events",
            "Reports",
            "Applications",
            "Tally",
            "Certificates",
        ]) {
            await expect(dialog.getByRole("checkbox", {name})).not.toBeChecked()
        }
        expect(dialog.queryByText(/password protected anyway/)).toBeNull()
        expect(graphql.calls).toEqual([])
        expect(args.setOpenExport).not.toHaveBeenCalled()
    },
}

export const Closed: Story = {
    args: {open: false},
    play: async () => {
        expect(within(document.body).queryByRole("dialog")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const Cancel: Story = {
    play: async ({args}) => {
        const dialog = await exportDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(args.setOpenExport).toHaveBeenCalledWith(false)
        expect(args.setLoadingExport).not.toHaveBeenCalled()
        expect(graphql.calls).toEqual([])
    },
}

export const ExportStartsATask: Story = {
    parameters: {expectedFailure: successChipDefect},
    play: async ({args}) => {
        const dialog = await exportDialog()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Include Voters"}))
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(args.setLoadingExport).toHaveBeenLastCalledWith(false))
        expect(args.setLoadingExport).toHaveBeenCalledWith(true)
        expect(args.setOpenExport).toHaveBeenCalledWith(false)
        expect(exportCall()).toEqual({
            name: "ExportElectionEvent",
            variables: {
                electionEventId: EVENT_ID,
                exportConfigurations: {...NO_OPTIONS, include_voters: true},
            },
            headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_READ},
        })
        // The task widget follows the started export.
        await expect(
            await within(document.body).findByText("Export Election Event", {
                selector: "p, span, h6",
            })
        ).toBeVisible()
        await waitFor(() =>
            expect(graphql.calls.find(({name}) => name === "GetTaskById")?.variables).toMatchObject(
                {task_id: TASK_ID}
            )
        )
        expect(within(document.body).queryByRole("dialog", {name: "Password"})).toBeNull()
    },
}

export const BulletinBoardForcesEncryption: Story = {
    parameters: {
        expectedFailure: {
            reason:
                "The generated password's read-only fields have no label, and the task " +
                "widget's success chip has white text below 4.5 contrast.",
            a11y: ["color-contrast", "label"],
        },
    },
    play: async () => {
        const dialog = await exportDialog()
        // Tally data is only exported with its bulletin board.
        await userEvent.click(dialog.getByRole("checkbox", {name: "Tally"}))
        await expect(dialog.getByRole("checkbox", {name: "Bulletin Board"})).toBeChecked()
        await expect(dialog.getByText(/password protected anyway/)).toBeVisible()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Bulletin Board"}))
        await expect(dialog.getByRole("checkbox", {name: "Tally"})).not.toBeChecked()
        await userEvent.click(dialog.getByRole("checkbox", {name: "Bulletin Board"}))
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        const password = within(
            await within(document.body).findByRole("dialog", {name: "Password"})
        )
        await expect(password.getByDisplayValue(PASSWORD)).toBeVisible()
        expect(exportCall()?.variables.exportConfigurations).toEqual({
            ...NO_OPTIONS,
            is_encrypted: true,
            bulletin_board: true,
        })
    },
}

export const ExportFailure: Story = {
    args: {outcome: "failure"},
    play: async ({args}) => {
        const dialog = await exportDialog()
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() => expect(args.setLoadingExport).toHaveBeenLastCalledWith(false))
        expect(graphql.calls.map(({name}) => name)).toEqual(["ExportElectionEvent"])
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
    },
}
