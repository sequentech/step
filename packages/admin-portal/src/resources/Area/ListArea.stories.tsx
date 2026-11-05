// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {EElectionEventWeightedVotingPolicy, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    STORY_IDS,
    areaRecords,
    contestRecord,
    eventPresentation,
    eventRecord,
} from "@/__stories__/fixtures"
import {ListArea} from "./ListArea"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** What reading the areas does. */
    reads: ReadState
    /** Whether the event has areas. */
    empty: boolean
    /** The event's weighted voting policy. */
    weightedVoting: EElectionEventWeightedVotingPolicy
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({weightedVoting}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const event = eventRecord(undefined, {
        presentation: {...eventPresentation, weighted_voting_policy: weightedVoting},
    })
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <RecordContextProvider value={event}>
                <ListArea />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const listDefects = {
    expectedFailure: {
        reason: "React-admin row selection labels a MUI 7 span instead of its checkbox and the row actions are unnamed icon buttons.",
        a11y: ["aria-prohibited-attr", "button-name", "label"],
    },
}

const meta = {
    title: "Admin/Area/ListArea",
    component: ListArea,
    args: {
        reads: "records",
        empty: false,
        weightedVoting: EElectionEventWeightedVotingPolicy.DISABLED_WEIGHTED_VOTING,
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        weightedVoting: {
            control: "select",
            options: Object.values(EElectionEventWeightedVotingPolicy),
        },
    },
    parameters: listDefects,
    beforeEach: async ({args}) => {
        const areas = args.empty
            ? []
            : areaRecords().map((area, index) => ({...area, annotations: {weight: index + 1}}))
        data = resourceBoundary(
            {sequent_backend_area: areas},
            {reads: {sequent_backend_area: args.reads}}
        )
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
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const areaRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const {permissions} = readStoryGlobals(globals)
        if ([EStoryPermissions.ADMIN, EStoryPermissions.ADMIN_LIGHT].includes(permissions)) {
            const north = await areaRow(canvasElement, "North district")
            // The contest chip comes from the area's own area-contest query.
            await expect(await within(north).findByText("Members")).toBeVisible()
            expect(within(north).queryByText("1")).not.toBeInTheDocument()
        } else {
            await expect(await canvas.findByText(i18n.t("areas.empty.header"))).toBeVisible()
        }
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_area")
        )
        expect(within(canvasElement).queryByRole("row", {name: /North district/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /North district/})).toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "The empty state's create and import buttons contain icon buttons.",
            a11y: ["nested-interactive"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Areas yet.")).toBeVisible()
        await expect(canvas.getByRole("button", {name: /Import/})).toBeVisible()
    },
}

export const WithoutAreaPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No Areas yet.")).toBeVisible()
        expect(canvas.queryByRole("button", {name: /Import/})).not.toBeInTheDocument()
        expect(data.calls).toEqual([])
    },
}

export const WeightedAreasShowTheirWeight: Story = {
    args: {weightedVoting: EElectionEventWeightedVotingPolicy.AREAS_WEIGHTED_VOTING},
    play: async ({canvasElement}) => {
        const south = await areaRow(canvasElement, "South district")
        await expect(within(south).getByText("2")).toBeVisible()
    },
}

export const DeleteAreaAfterConfirmation: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const south = await areaRow(canvasElement, "South district")
        const buttons = within(south).getAllByRole("button")
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: "Delete"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "delete",
                    resource: "sequent_backend_area",
                    params: expect.objectContaining({id: STORY_IDS.secondArea}),
                },
            ])
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /South district/})).toBeNull()
        )
        // Axe checks the page once the dialog has gone and the list is visible again.
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}
