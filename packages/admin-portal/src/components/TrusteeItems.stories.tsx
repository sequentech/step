// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {
    STORY_IDS,
    keysCeremonyRecord,
    storyId,
    tallySessionRecord,
    trusteeRecords,
} from "@/__stories__/fixtures"
import {TrusteeItems} from "./TrusteeItems"

/** A trustee of the tenant that the event's keys ceremony did not choose. */
const otherTrustee = {id: storyId(5, 9), name: "trustee4"}

type Props = React.ComponentProps<typeof TrusteeItems> & {
    /** Whether reading the keys ceremony answers or stays loading. */
    reads: Exclude<ReadState, "error">
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Components/TrusteeItems",
    component: TrusteeItems,
    args: {
        reads: "records",
        record: tallySessionRecord(),
        trusteeNames: [...trusteeRecords.map(({id, name}) => ({id, name})), otherTrustee],
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading"]},
        record: {table: {disable: true}},
    },
    beforeEach: ({args}) => {
        data = resourceBoundary(
            {sequent_backend_keys_ceremony: [keysCeremonyRecord()]},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({})
    },
    render: ({reads: _reads, ...props}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <TrusteeItems {...props} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

const keysCeremonyRead = () =>
    waitFor(() =>
        expect(data.calls).toEqual([
            {
                method: "getOne",
                args: [
                    "sequent_backend_keys_ceremony",
                    expect.objectContaining({id: STORY_IDS.keysCeremony}),
                ],
            },
        ])
    )

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        for (const {name} of trusteeRecords) {
            await expect(await canvas.findByText(String(name))).toBeVisible()
        }
        expect(canvas.queryByText(otherTrustee.name)).not.toBeInTheDocument()
        await keysCeremonyRead()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await keysCeremonyRead()
        expect(within(canvasElement).queryByText(String(trusteeRecords[0].name))).toBeNull()
    },
}

export const WithoutTrusteeNames: Story = {
    args: {trusteeNames: undefined},
    play: async ({canvasElement}) => {
        await keysCeremonyRead()
        expect(within(canvasElement).queryByText(String(trusteeRecords[0].name))).toBeNull()
    },
}
