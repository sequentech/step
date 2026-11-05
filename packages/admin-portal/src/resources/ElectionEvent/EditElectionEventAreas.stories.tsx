// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords, contestRecord, eventRecord} from "@/__stories__/fixtures"
import {EditElectionEventAreas} from "./EditElectionEventAreas"
import {paramsOf} from "./__stories__/ElectionEventFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the event has areas. */
    empty: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

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
                <EditElectionEventAreas />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventAreas",
    component: EditElectionEventAreas,
    args: {empty: false},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox and the row actions are unnamed icon buttons.",
            a11y: ["aria-prohibited-attr", "button-name", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({sequent_backend_area: args.empty ? [] : areaRecords()})
        graphql = graphqlBoundary(
            {
                get_area_with_area_contests: ({variables}) => ({
                    data: {
                        sequent_backend_area_contest:
                            variables.areaId === STORY_IDS.area
                                ? [{id: STORY_IDS.areaContest, contest: contestRecord()}]
                                : [],
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

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {permissions} = readStoryGlobals(globals)
        if (![EStoryPermissions.ADMIN, EStoryPermissions.ADMIN_LIGHT].includes(permissions)) {
            await expect(await canvas.findByText(i18n.t("areas.empty.header"))).toBeVisible()
            return
        }
        await expect(await canvas.findByRole("row", {name: /North district/})).toBeVisible()
        await expect(canvas.getByRole("row", {name: /South district/})).toBeVisible()
        // The areas are those of the event in the record context.
        expect(paramsOf(data, "getList", "sequent_backend_area")).toMatchObject({
            filter: {tenant_id: STORY_IDS.tenant, election_event_id: STORY_IDS.event},
        })
    },
}

export const Empty: Story = {
    args: {empty: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "The empty state's create and import buttons contain icon buttons.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(i18n.t("areas.empty.header"))).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("row")).toBeNull())
    },
}

export const WithoutAreaPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(i18n.t("areas.empty.header"))
        ).toBeVisible()
        expect(data.calls).toEqual([])
    },
}
