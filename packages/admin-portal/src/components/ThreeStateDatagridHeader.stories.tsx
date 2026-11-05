// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {Datagrid, List, TextField, type DatagridHeaderProps} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {areaRecords, storyId} from "@/__stories__/fixtures"
import {ThreeStateDatagridHeader} from "./ThreeStateDatagridHeader"

// Listed in an order that differs from both sort directions, so each state is visible.
const [north, south] = areaRecords()
const areas = [
    north,
    south,
    {...north, id: storyId(7, 3), name: "East district", description: "East district voters"},
]

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Components/ThreeStateDatagridHeader",
    component: ThreeStateDatagridHeader,
    args: {},
    beforeEach: () => {
        boundary = graphqlBoundary({})
        data = resourceBoundary({sequent_backend_area: areas})
    },
    render: (args) => {
        const Header = (headerProps: DatagridHeaderProps) => (
            <ThreeStateDatagridHeader {...headerProps} {...args} />
        )
        return (
            <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
                <List resource="sequent_backend_area" actions={false} disableSyncWithLocation>
                    <Datagrid header={Header} bulkActionButtons={false} rowClick={false}>
                        <TextField source="name" />
                        <TextField source="type" />
                    </Datagrid>
                </List>
            </AdminStoryProvider>
        )
    },
} satisfies Meta<typeof ThreeStateDatagridHeader>
export default meta
type Story = StoryObj<typeof meta>

const names = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("cell")[0].textContent)

const lastSort = () =>
    data.calls.filter(({method}) => method === "getList").at(-1)?.args[1] as {
        sort: {field: string; order: string}
    }

async function sortBy(canvasElement: HTMLElement, column: string) {
    const requests = data.calls.length
    // The sort button is named after its next order, e.g. "Sort by name ascending".
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: new RegExp(`sort by ${column}`, "i")})
    )
    await waitFor(() => expect(data.calls.length).toBeGreaterThan(requests))
}

export const SortCyclesAscendingDescendingThenUnsorted: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("East district")
        expect(names(canvasElement)).toEqual(["North district", "South district", "East district"])

        await sortBy(canvasElement, "Name")
        expect(lastSort().sort).toEqual({field: "name", order: "ASC"})
        await waitFor(() =>
            expect(names(canvasElement)).toEqual([
                "East district",
                "North district",
                "South district",
            ])
        )

        await sortBy(canvasElement, "Name")
        expect(lastSort().sort).toEqual({field: "name", order: "DESC"})
        await waitFor(() =>
            expect(names(canvasElement)).toEqual([
                "South district",
                "North district",
                "East district",
            ])
        )

        // A third click drops the column's sort instead of flipping back to ascending,
        // so the list requests its own default order again (react-admin's id ascending).
        await sortBy(canvasElement, "Name")
        expect(lastSort().sort).toEqual({field: "id", order: "ASC"})
        await waitFor(() =>
            expect(names(canvasElement)).toEqual([
                "North district",
                "South district",
                "East district",
            ])
        )
    },
}

export const AnotherColumnStartsAscending: Story = {
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("East district")
        await sortBy(canvasElement, "Name")
        await sortBy(canvasElement, "Name")
        expect(lastSort().sort).toEqual({field: "name", order: "DESC"})
        await sortBy(canvasElement, "Type")
        expect(lastSort().sort).toEqual({field: "type", order: "ASC"})
    },
}

export const ResetsToTheGivenDefaultSort: Story = {
    args: {defaultSort: {field: "name", order: "DESC"}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("East district")
        await sortBy(canvasElement, "Type")
        await sortBy(canvasElement, "Type")
        await sortBy(canvasElement, "Type")
        expect(lastSort().sort).toEqual({field: "name", order: "DESC"})
        await waitFor(() =>
            expect(names(canvasElement)).toEqual([
                "South district",
                "North district",
                "East district",
            ])
        )
    },
}
