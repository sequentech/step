// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import {ElectoralLog} from "./ElectoralLog"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <RecordContextProvider value={eventRecord()}>
                <ElectoralLog />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const noPermission = "You don't have permission to access logs."

// The electoral log list has its own section; its reads stay loading here.
const meta = {
    title: "Admin/Election event/ElectoralLog",
    component: ElectoralLog,
    beforeEach: async () => {
        data = recordsOrPending()
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        await graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies Meta<typeof ElectoralLog>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {permissions} = readStoryGlobals(globals)
        if ([EStoryPermissions.ADMIN_LOCKDOWN, EStoryPermissions.NONE].includes(permissions)) {
            await expect(await canvas.findByText(noPermission)).toBeVisible()
            return
        }
        await expect(await canvas.findByText("Logs")).toBeVisible()
        // The log of the event in the record context.
        await waitFor(() =>
            expect(paramsOf(data, "getList", "electoral_log")).toMatchObject({
                filter: {election_event_id: EVENT_ID},
            })
        )
        expect(canvas.queryByText(noPermission)).toBeNull()
    },
}

export const WithoutLogPermission: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LOCKDOWN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(noPermission)).toBeVisible()
        expect(canvas.queryByText("Logs")).toBeNull()
        expect(data.calls).toEqual([])
    },
}
