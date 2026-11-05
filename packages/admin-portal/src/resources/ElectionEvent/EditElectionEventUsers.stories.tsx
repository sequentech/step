// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, electionRecord, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EditElectionEventUsers} from "./EditElectionEventUsers"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** The record in context: the election event, or one of its elections. */
    scope: "event" | "election"
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture({scope}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <RecordContextProvider value={scope === "event" ? eventRecord() : electionRecord()}>
                <EditElectionEventUsers />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

// The voter list has its own section; its reads stay loading here.
const meta = {
    title: "Admin/Election event/EditElectionEventUsers",
    component: EditElectionEventUsers,
    args: {scope: "event"},
    argTypes: {scope: {control: "inline-radio", options: ["event", "election"]}},
    beforeEach: async () => {
        data = recordsOrPending()
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const userFilter = async () => {
    await waitFor(() => expect(paramsOf(data, "getList", "user")).toBeDefined())
    return (paramsOf(data, "getList", "user") as {filter: Record<string, unknown>}).filter
}

export const Populated: Story = {
    play: async ({globals}) => {
        const {permissions} = readStoryGlobals(globals)
        if (![EStoryPermissions.ADMIN, EStoryPermissions.ADMIN_LIGHT].includes(permissions)) {
            expect(data.calls).toEqual([])
            return
        }
        // The voters of the whole event, not of one of its elections.
        const filter = await userFilter()
        expect(filter).toMatchObject({tenant_id: STORY_IDS.tenant, election_event_id: EVENT_ID})
        expect(filter.election_id).toBeUndefined()
    },
}

export const ElectionVoters: Story = {
    args: {scope: "election"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async () => {
        await expect(await userFilter()).toMatchObject({
            election_event_id: EVENT_ID,
            election_id: STORY_IDS.election,
        })
    },
}

export const WithoutVoterPermission: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LOCKDOWN},
    play: async ({canvasElement}) => {
        // Nothing renders but the story router's current location.
        const location = within(canvasElement).getByLabelText("Current location")
        expect(canvasElement.textContent).toBe(location.textContent)
        expect(data.calls).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}
