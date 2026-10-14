// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    CreateFlowStory,
    UUID,
    createFlow,
    type CreateFlow,
} from "@/components/election-event/create/__stories__/CreateElectionEventFixture"
import {ElectionEventListContent} from "./ElectionEventList"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

interface Scenario {
    /** Whether the tenant has an election event. */
    empty: boolean
}

let flow: CreateFlow

// The application's create provider surrounds the content, as ElectionEventList does.
function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <ElectionEventListContent />
        </CreateFlowStory>
    )
}

const meta = {
    title: "Admin/Election event/ElectionEventListContent",
    component: ElectionEventListContent,
    args: {empty: true},
    beforeEach: async ({args}) => {
        flow = createFlow({events: args.empty ? [] : [eventRecord()]})
        await flow.graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Current location")

export const Populated: Story = {
    args: {empty: false},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(location(canvasElement).textContent).toBe(`/${RESOURCE}/${EVENT_ID}`)
        )
        expect(
            flow.data.calls.find(({method, args}) => method === "getList" && args[0] === RESOURCE)
                ?.args[1]
        ).toMatchObject({filter: {tenant_id: TENANT_ID, is_archived: false}})
    },
}

export const Empty: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("electionEventScreen.error.noResult"))
        ).toBeVisible()
        await expect(canvas.getByText(i18n.t("common.resources.noResult.askCreate"))).toBeVisible()
        expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`)
    },
}

export const CreateTheFirstEvent: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await userEvent.click(
            await within(canvasElement).findByRole("button", {name: i18n.t("common.label.add")})
        )
        const body = within(document.body)
        await waitFor(() => expect(body.getByText("Create an Election Event")).toBeVisible())
        await userEvent.type(body.getByRole("textbox", {name: "Name"}), "Council event")
        await userEvent.click(body.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(
                flow.graphql.calls.find(({name}) => name === "CreateElectionEvent")?.variables
            ).toMatchObject({
                electionEvent: {id: expect.stringMatching(UUID), name: "Council event"},
            })
        )
        const notice = await body.findByText("Election Event created")
        await waitFor(() => expect(notice).toBeVisible())
        await waitFor(() => expect(body.queryByText("Create an Election Event")).toBeNull())
    },
}
