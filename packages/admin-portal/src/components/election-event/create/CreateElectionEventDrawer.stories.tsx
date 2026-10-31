// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {CreateDataDrawer} from "./CreateElectionEventDrawer"
import {
    CreateFlowStory,
    createFlow,
    type CreateFlow,
} from "./__stories__/CreateElectionEventFixture"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the provider's create drawer has been opened, as the event list's Add does. */
    opened: boolean
}

let flow: CreateFlow

function OpenDrawer({opened}: Scenario) {
    const {openCreateDrawer} = useCreateElectionEventStore()
    useEffect(() => {
        if (opened) openCreateDrawer()
    }, [opened])
    return null
}

function Fixture(args: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <OpenDrawer {...args} />
            <CreateDataDrawer />
        </CreateFlowStory>
    )
}

const meta = {
    title: "Admin/Election event/Create/CreateDataDrawer",
    component: CreateDataDrawer,
    args: {opened: true},
    beforeEach: async () => {
        flow = createFlow()
        await flow.graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const drawer = async () => {
    const form = await within(document.body).findByText("Create an Election Event")
    await waitFor(() => expect(form).toBeVisible())
    return within(document.body)
}

const drawerClosed = () =>
    waitFor(() => expect(within(document.body).queryByText("Create an Election Event")).toBeNull())

export const Populated: Story = {
    parameters: {
        expectedFailure: {
            reason: "The drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async () => {
        const screen = await drawer()
        await expect(screen.getByRole("textbox", {name: "Name"})).toBeVisible()
        await expect(screen.getByRole("textbox", {name: "Description"})).toBeVisible()
    },
}

export const Closed: Story = {
    args: {opened: false},
    play: async () => {
        expect(within(document.body).queryByText("Create an Election Event")).toBeNull()
        expect(flow.graphql.calls).toEqual([])
    },
}

export const CloseWithEscape: Story = {
    play: async () => {
        const screen = await drawer()
        await userEvent.type(screen.getByRole("textbox", {name: "Name"}), "Council event")
        await userEvent.keyboard("{Escape}")
        await drawerClosed()
        expect(flow.graphql.calls.map(({name}) => name)).not.toContain("CreateElectionEvent")
    },
}

export const SavingClosesTheDrawer: Story = {
    play: async () => {
        const screen = await drawer()
        await userEvent.type(screen.getByRole("textbox", {name: "Name"}), "Council event")
        await userEvent.click(screen.getByRole("button", {name: "Save"}))
        await drawerClosed()
        await waitFor(() =>
            expect(flow.graphql.calls.map(({name}) => name)).toContain("CreateElectionEvent")
        )
        await expect(await within(document.body).findByText("Election Event created")).toBeVisible()
    },
}
