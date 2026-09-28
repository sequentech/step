// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETasksExecution} from "@/types/tasksExecution"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import type {WidgetProps} from "./Widget"
import {TASK_ID, taskRecord, widgetDefects} from "./__stories__/WidgetFixture"
import {WidgetsStack} from "./WidgetsStack"

interface Scenario {
    /** Called with the identifier of the widget whose close button was pressed. */
    onClose: (identifier: string) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const widget = (
    identifier: string,
    type: ETasksExecution,
    status: ETaskExecutionStatus,
    taskId?: string
): Omit<WidgetProps, "onClose"> => ({identifier, type, status, taskId})

const initialWidgets = [
    widget(
        "widget_export",
        ETasksExecution.EXPORT_ELECTION_EVENT,
        ETaskExecutionStatus.IN_PROGRESS,
        TASK_ID
    ),
    widget("widget_import", ETasksExecution.IMPORT_USERS, ETaskExecutionStatus.FAILED),
]

/** Owns the widgets map, as WidgetsContextProvider does, so a closed widget leaves the stack. */
function Fixture({onClose}: Scenario) {
    const [widgets, setWidgets] = useState(
        () =>
            new Map(
                initialWidgets.map((props): [string, WidgetProps] => [
                    props.identifier,
                    {
                        ...props,
                        onClose: (identifier) => {
                            onClose(identifier)
                            setWidgets((current) => {
                                const next = new Map(current)
                                next.delete(identifier)
                                return next
                            })
                        },
                    },
                ])
            )
    )
    return (
        <AdminStoryProvider boundary={boundary} role={EStoryPermissions.ADMIN}>
            <WidgetsStack widgetsMap={widgets} />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/WidgetsStack",
    component: WidgetsStack,
    args: {onClose: fn()},
    parameters: widgetDefects("button-name", "nested-interactive", "color-contrast"),
    beforeEach: async () => {
        boundary = graphqlBoundary(
            {
                GetTaskById: () => ({
                    data: {
                        sequent_backend_tasks_execution: [taskRecord(ETaskExecutionStatus.SUCCESS)],
                    },
                }),
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const closeButton = (container: HTMLElement) => {
    const button = within(container).getByTestId("CloseIcon").closest("button")
    if (!button) throw new Error("The widget has no close button")
    return button
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Task: Export Election Event")).toBeVisible()
        await expect(canvas.getByText("Task: Import Users")).toBeVisible()
        // The export widget follows its task, which has completed.
        await expect(await canvas.findByText("SUCCESS")).toBeVisible()
        await expect(canvas.getByText("FAILED")).toBeVisible()
        expect(boundary.calls).toEqual([
            {name: "GetTaskById", variables: {task_id: TASK_ID}, headers: {}},
        ])
    },
}

export const CloseOneWidget: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const imported = await canvas.findByText("Task: Import Users")
        const container = imported.closest<HTMLElement>(".widget-container")
        if (!container) throw new Error("The import widget is missing")
        await userEvent.click(closeButton(container))
        expect(args.onClose).toHaveBeenCalledWith("widget_import")
        await waitFor(() => expect(canvas.queryByText("Task: Import Users")).toBeNull())
        await expect(canvas.getByText("Task: Export Election Event")).toBeVisible()
    },
}
