// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ETaskExecutionStatus, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {taskRecord} from "@/components/__stories__/WidgetFixture"
import {ETasksExecution} from "@/types/tasksExecution"
import {CreateTenant} from "./CreateTenant"
import {TENANT_RESOURCE, tenantRecords} from "./__stories__/TenantFixture"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"

/** How the tenant service answers the creation. */
type Reply = "created" | "rejected" | "failure"

interface Scenario {
    reply: Reply
    /** Whether a parent drawer owns the open state; otherwise closing returns to the list. */
    inDrawer: boolean
    setIsDrawerOpen: (open: boolean) => void
}

const NEW_TENANT_ID = storyId(1, 3)
const CREATION_TASK_ID = storyId(9, 4)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Tenant/CreateTenant",
    component: CreateTenant,
    args: {reply: "created", inDrawer: true, setIsDrawerOpen: fn()},
    argTypes: {reply: {control: "inline-radio", options: ["created", "rejected", "failure"]}},
    parameters: {
        expectedFailure: {
            reason: "The drawer dialog has no accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: tenantRecords()})
        graphql = graphqlBoundary(
            {
                InsertTenant: ({variables}) => {
                    if (args.reply === "failure") {
                        return {errors: [new GraphQLError("Synthetic tenant service failure")]}
                    }
                    return {
                        data: {
                            insertTenant:
                                args.reply === "rejected"
                                    ? {
                                          id: NEW_TENANT_ID,
                                          slug: variables.slug,
                                          error_msg: "Slug already in use",
                                          task_execution: null,
                                      }
                                    : {
                                          id: NEW_TENANT_ID,
                                          slug: variables.slug,
                                          error_msg: null,
                                          task_execution: taskRecord(
                                              ETaskExecutionStatus.IN_PROGRESS,
                                              {
                                                  id: CREATION_TASK_ID,
                                                  name: "Create tenant",
                                                  type: ETasksExecution.CREATE_TEMANT,
                                              }
                                          ),
                                      },
                        },
                    }
                },
                GetTaskById: () => ({
                    data: {
                        sequent_backend_tasks_execution: [
                            taskRecord(ETaskExecutionStatus.SUCCESS, {
                                id: CREATION_TASK_ID,
                                name: "Create tenant",
                                type: ETasksExecution.CREATE_TEMANT,
                                annotations: {},
                            }),
                        ],
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({inDrawer, setIsDrawerOpen}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={EStoryPermissions.ADMIN}
        >
            <WidgetsContextProvider>
                <CreateTenant setIsDrawerOpen={inDrawer ? setIsDrawerOpen : undefined} />
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const taskTitle = () =>
    i18n.t("tasksScreen.widget.taskTitle", {
        title: i18n.t(`tasksScreen.tasksExecution.${ETasksExecution.CREATE_TEMANT}`),
    })

async function createTenant(slug: string) {
    const drawer = within(await within(document.body).findByRole("dialog"))
    await userEvent.type(await drawer.findByRole("textbox", {name: "Slug"}), slug)
    await userEvent.click(drawer.getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async () => {
        const drawer = within(await within(document.body).findByRole("dialog"))
        await expect(drawer.getByText(i18n.t("tenantScreen.new.subtitle"))).toBeVisible()
        // The heading names the signed-in administrator's tenant, which the form reads.
        await expect(
            await drawer.findByRole("heading", {
                name: `${i18n.t("tenantScreen.common.title")} example-council`,
            })
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
        expect(data.calls[0].args[1]).toMatchObject({id: TENANT_ID})
    },
}

export const CreateATenant: Story = {
    parameters: {
        expectedFailure: {
            reason: "The drawer dialog has no accessible name; the task widget's success chip has a white label below the contrast minimum.",
            a11y: ["aria-dialog-name", "color-contrast"],
        },
    },
    play: async ({args}) => {
        await createTenant("new-council")
        await waitFor(() => expect(args.setIsDrawerOpen).toHaveBeenCalledWith(false))
        expect(graphql.calls[0]).toEqual({
            name: "InsertTenant",
            variables: {slug: "new-council"},
            headers: {},
        })
        // The creation continues as a task that the widget follows.
        const widget = await within(document.body).findByText(taskTitle())
        await waitFor(() => expect(widget).toBeVisible())
        await waitFor(() =>
            expect(graphql.calls.map(({name, variables}) => [name, variables])).toContainEqual([
                "GetTaskById",
                {task_id: CREATION_TASK_ID},
            ])
        )
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const CreationRejected: Story = {
    args: {reply: "rejected"},
    play: async ({args}) => {
        await createTenant("example-council")
        await expect(await within(document.body).findByText(taskTitle())).toBeVisible()
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["InsertTenant"])
        expect(args.setIsDrawerOpen).not.toHaveBeenCalled()
        await expect(within(document.body).getByRole("button", {name: "Save"})).toBeEnabled()
    },
}

export const CreationFailure: Story = {
    args: {reply: "failure"},
    play: async ({args}) => {
        await createTenant("new-council")
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(graphql.calls.map(({name}) => name)).toEqual(["InsertTenant"])
        expect(args.setIsDrawerOpen).not.toHaveBeenCalled()
    },
}

export const CloseReturnsToTheList: Story = {
    args: {inDrawer: false},
    play: async ({canvasElement}) => {
        await within(document.body).findByRole("dialog")
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(
                // The open drawer hides the page from assistive technology.
                within(canvasElement).getByRole("status", {name: "Current location", hidden: true})
            ).toHaveTextContent(`/${TENANT_RESOURCE}`)
        )
        expect(graphql.calls).toEqual([])
    },
}
