// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTallyContextProvider} from "@/providers/ElectionEventTallyProvider"
import {TASK_ID, taskRecord, taskRecords} from "@/resources/Tasks/__stories__/TasksFixture"
import {EditElectionEventTasks} from "./EditElectionEventTasks"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** What reading the task executions does. */
    reads: ReadState
    empty: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

/** The event tabs pass a new `showList` each time the Tasks tab is chosen again. */
function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    const [showList, setShowList] = useState<string>()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <ElectionEventTallyContextProvider>
                <button type="button" onClick={() => setShowList(String(Date.now()))}>
                    Tasks tab
                </button>
                <RecordContextProvider value={eventRecord()}>
                    <EditElectionEventTasks showList={showList} />
                </RecordContextProvider>
            </ElectionEventTallyContextProvider>
        </AdminStoryProvider>
    )
}

const chipContrast = "White status chip labels lack contrast on the success and warning colours."

const meta = {
    title: "Admin/Election event/EditElectionEventTasks",
    component: EditElectionEventTasks,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: `The row's view action is an icon button without an accessible name. ${chipContrast}`,
            a11y: ["button-name", "color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {sequent_backend_tasks_execution: args.empty ? [] : taskRecords()},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(
            {
                GetTaskById: ({variables}) => ({
                    data: {
                        sequent_backend_tasks_execution: [
                            taskRecord({id: String(variables.task_id)}),
                        ],
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const title = () => i18n.t("tasksScreen.title")
const exportRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: /Export voters/})

async function viewExportTask(canvasElement: HTMLElement) {
    const row = await exportRow(canvasElement)
    await userEvent.click(within(row).getByRole("button"))
    const details = await within(canvasElement).findByRole("table", {name: "task details table"})
    await expect(within(details).getByText("admin")).toBeVisible()
    expect(graphql.calls.map(({name, variables}) => [name, variables])).toEqual([
        ["GetTaskById", {task_id: TASK_ID}],
    ])
}

const detailsGone = (canvasElement: HTMLElement) =>
    waitFor(() =>
        expect(within(canvasElement).queryByRole("table", {name: "task details table"})).toBeNull()
    )

export const Populated: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(title())).toBeVisible()
        await expect(await exportRow(canvasElement)).toBeVisible()
        await expect(canvas.getByRole("row", {name: /Import users/})).toBeVisible()
        expect(data.calls.find(({method}) => method === "getList")?.args[1]).toMatchObject({
            filter: {election_event_id: STORY_IDS.event},
        })
        expect(graphql.calls).toEqual([])
    },
}

export const Empty: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is light grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("No Sequent backend tasks executions yet.")
        ).toBeVisible()
        expect(canvas.queryByRole("row")).toBeNull()
        await expect(canvas.getByText(title())).toBeVisible()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Export voters/})).toBeNull()
    },
}

export const ViewATaskAndGoBack: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await viewExportTask(canvasElement)
        const canvas = within(canvasElement)
        expect(canvas.queryByRole("row", {name: /Import users/})).toBeNull()
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.back")}))
        await detailsGone(canvasElement)
        await expect(await exportRow(canvasElement)).toBeVisible()
    },
}

export const ChoosingTheTabAgainShowsTheList: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await viewExportTask(canvasElement)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Tasks tab"}))
        await detailsGone(canvasElement)
        await expect(await exportRow(canvasElement)).toBeVisible()
    },
}
