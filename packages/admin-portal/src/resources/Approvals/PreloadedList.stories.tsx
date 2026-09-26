// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {ListBase, WithListContext} from "react-admin"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {PreloadedList, type FilterValues} from "./PreloadedList"
import {
    ApprovalsScreen,
    listFilters,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    defaultFilters: FilterValues | undefined
}

const meta = {
    title: "Admin/Approvals/PreloadedList",
    component: PreloadedList,
    args: {
        reads: "records",
        empty: false,
        defaultFilters: {first_name: {IsLike: "Alicia"}},
    },
    beforeEach: ({args}) => setUpApprovals(args),
    render: ({defaultFilters}) => (
        <ApprovalsScreen>
            <ListBase resource="user" disableSyncWithLocation storeKey={false}>
                <PreloadedList defaultFilters={defaultFilters} resource="user">
                    <WithListContext
                        render={({data, filterValues}) => (
                            <>
                                <output aria-label="Filters">{JSON.stringify(filterValues)}</output>
                                <ul aria-label="Voters">
                                    {data?.map((voter) => (
                                        <li key={voter.id}>{voter.email}</li>
                                    ))}
                                </ul>
                            </>
                        )}
                    />
                </PreloadedList>
            </ListBase>
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Filters"})).toHaveTextContent(
                JSON.stringify({first_name: {IsLike: "Alicia"}})
            )
        )
        await waitFor(() =>
            expect(listFilters("user").at(-1)).toEqual({first_name: {IsLike: "Alicia"}})
        )
        await expect(canvas.getByRole("list", {name: "Voters"})).toBeVisible()
    },
}

export const WithoutDefaults: Story = {
    args: {defaultFilters: undefined},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("alicia@example.test")).toBeVisible()
        await expect(canvas.getByRole("status", {name: "Filters"})).toHaveTextContent("{}")
        expect(listFilters("user")).toEqual([{}])
    },
}
