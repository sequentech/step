// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {LANGUAGE_CONF} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CreateElectionEventScreen} from "./CreateScreen"
import {
    UUID,
    CreateFlowStory,
    createFlow,
    type CreateFlow,
} from "./__stories__/CreateElectionEventFixture"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the creation service fails. */
    createFailure: boolean
}

let flow: CreateFlow

function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <CreateElectionEventScreen />
        </CreateFlowStory>
    )
}

const meta = {
    title: "Admin/Election event/Create/CreateElectionEventScreen",
    component: CreateElectionEventScreen,
    args: {createFailure: false},
    beforeEach: async ({args}) => {
        flow = createFlow({createFailure: args.createFailure})
        await flow.graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const creations = () => flow.graphql.calls.filter(({name}) => name === "CreateElectionEvent")

async function create(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Council event")
    await userEvent.type(canvas.getByRole("textbox", {name: "Description"}), "Annual election")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
    await waitFor(() => expect(creations()).toHaveLength(1))
    return creations()[0].variables.electionEvent as Record<string, unknown>
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Create an Election Event")).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("")
        await expect(canvas.getByRole("textbox", {name: "Description"})).toBeVisible()
        // The provider reads the tenant, whose languages a new event gets.
        await waitFor(() =>
            expect(flow.data.calls).toContainEqual({
                method: "getOne",
                args: ["sequent_backend_tenant", expect.objectContaining({id: TENANT_ID})],
            })
        )
        expect(creations()).toEqual([])
    },
}

export const CreateAndOpenTheEvent: Story = {
    play: async ({canvasElement}) => {
        const event = await create(canvasElement)
        expect(event).toMatchObject({
            id: expect.stringMatching(UUID),
            name: "Council event",
            description: "Annual election",
            encryption_protocol: "RSA256",
            tenant_id: TENANT_ID,
            is_archived: false,
            presentation: {
                language_conf: LANGUAGE_CONF,
                i18n: {en: expect.objectContaining({name: "Council event"})},
            },
        })
        await expect(await within(document.body).findByText("Election Event created")).toBeVisible()
        const location = within(canvasElement).getByLabelText("Current location")
        await waitFor(() =>
            expect(location).toHaveTextContent(`/sequent_backend_election_event/${event.id}`)
        )
        expect(flow.created).toHaveBeenCalledWith({
            id: event.id,
            type: "sequent_backend_election_event",
        })
    },
}

export const CreationFailure: Story = {
    args: {createFailure: true},
    play: async ({canvasElement}) => {
        await create(canvasElement)
        const canvas = within(canvasElement)
        // The failure reaches only the task widget; the form stays where it was.
        await waitFor(() => expect(canvas.getByRole("button", {name: "Save"})).toBeEnabled())
        expect(canvas.queryByRole("progressbar")).toBeNull()
        expect(canvas.getByLabelText("Current location")).toHaveTextContent(/^\/$/)
        expect(flow.created).not.toHaveBeenCalled()
        expect(within(document.body).queryByText("Election Event created")).toBeNull()
    },
}
