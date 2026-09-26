// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MenuItem} from "@mui/material"
import {ListContextProvider, useList} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CustomActionsMenu} from "./CustomActionsMenu"

interface Scenario {
    open: boolean
    handleCloseCustomMenu: () => void
    doContext: (context: {total?: number; resource: string}) => void
    pickFilter: (name: string) => void
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({open, handleCloseCustomMenu, doContext, pickFilter}: Scenario) {
    const list = useList({
        data: [
            {id: "1", username: "alice"},
            {id: "2", username: "bob"},
        ],
        resource: "user",
    })
    const [anchor, setAnchor] = React.useState<HTMLElement | null>(null)
    return (
        <AdminStoryProvider boundary={graphql}>
            <ListContextProvider value={list}>
                <button ref={setAnchor} type="button">
                    Filters
                </button>
                {anchor && (
                    <CustomActionsMenu
                        anchorEl={anchor}
                        open={open}
                        handleCloseCustomMenu={handleCloseCustomMenu}
                        doContext={doContext}
                        customFiltersList={["Has voted", "Not enrolled"].map((name) => (
                            <MenuItem key={name} onClick={() => pickFilter(name)}>
                                {name}
                            </MenuItem>
                        ))}
                    />
                )}
            </ListContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/User/CustomActionsMenu",
    component: CustomActionsMenu,
    args: {open: true, handleCloseCustomMenu: fn(), doContext: fn(), pickFilter: fn()},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({args}) => {
        // React-admin's Menu is the sidebar list, so the items show whatever `open` is.
        const menu = within(await within(document.body).findByRole("menu"))
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            "Has voted",
            "Not enrolled",
        ])
        // The menu hands its list context to the owner once, on mount.
        await waitFor(() =>
            expect(args.doContext).toHaveBeenCalledWith(
                expect.objectContaining({
                    total: 2,
                    data: [
                        expect.objectContaining({username: "alice"}),
                        expect.objectContaining({username: "bob"}),
                    ],
                })
            )
        )
        expect(args.doContext).toHaveBeenCalledTimes(1)
        expect(graphql.calls).toEqual([])
    },
}

export const PickAFilter: Story = {
    play: async ({args}) => {
        await userEvent.click(
            await within(document.body).findByRole("menuitem", {name: "Not enrolled"})
        )
        expect(args.pickFilter).toHaveBeenCalledWith("Not enrolled")
    },
}
