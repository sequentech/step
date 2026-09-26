// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import type {Identifier, RaRecord} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, areaRecords} from "@/__stories__/fixtures"
import {AreaItem} from "./AreaItem"
import {QueryStateProbe, readStatuses} from "./__stories__/QueryStateFixture"

interface Scenario {
    /** What reading the area does. */
    reads: ReadState
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Components/AreaItem",
    component: AreaItem,
    args: {reads: "records"},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({sequent_backend_area: areaRecords()}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <QueryStateProbe />
            {/* Tally sheet lists pass the area ID, which AreaItem reads as its `record`. */}
            <AreaItem record={STORY_IDS.secondArea as unknown as RaRecord<Identifier>} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const readArea = () =>
    waitFor(() =>
        expect(data.calls).toEqual([
            {
                method: "getOne",
                args: ["sequent_backend_area", expect.objectContaining({id: STORY_IDS.secondArea})],
            },
        ])
    )
const chips = (canvasElement: HTMLElement) => canvasElement.querySelectorAll(".MuiChip-root")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("South district")).toBeVisible()
        await readArea()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await readArea()
        expect(readStatuses()).toEqual(["pending"])
        expect(chips(canvasElement)).toHaveLength(0)
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await readArea()
        await waitFor(() => expect(readStatuses()).toEqual(["error"]))
        // The failure is silent: no chip and no message.
        expect(chips(canvasElement)).toHaveLength(0)
        expect(within(canvasElement).queryByText(/unavailable/)).not.toBeInTheDocument()
    },
}
