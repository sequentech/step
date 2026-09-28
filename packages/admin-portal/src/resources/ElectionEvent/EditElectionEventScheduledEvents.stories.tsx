// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditElectionEventEvents} from "./EditElectionEventScheduledEvents"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

type Props = React.ComponentProps<typeof EditElectionEventEvents>

function Fixture(props: Props) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <EditElectionEventEvents {...props} />
        </AdminStoryProvider>
    )
}

// The scheduled events list has its own section; its reads stay loading here.
const meta = {
    title: "Admin/Election event/EditElectionEventEvents",
    component: EditElectionEventEvents,
    args: {electionEventId: EVENT_ID},
    beforeEach: async () => {
        data = recordsOrPending()
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        // Every role group of the realm template may read the election event.
        if (readStoryGlobals(globals).permissions === EStoryPermissions.NONE) {
            expect(canvas.queryByText("Scheduled Events")).toBeNull()
            return
        }
        await expect(await canvas.findByText("Scheduled Events")).toBeVisible()
        await waitFor(() =>
            expect(paramsOf(data, "getList", "sequent_backend_scheduled_event")).toMatchObject({
                filter: {election_event_id: EVENT_ID, tenant_id: STORY_IDS.tenant},
            })
        )
    },
}

export const WithoutReadPermission: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByText("Scheduled Events")).toBeNull()
        expect(data.calls).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}
