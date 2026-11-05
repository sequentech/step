// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {memoryStore, useSidebarState} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {fullAdminTheme} from "@/services/AdminTheme"
import {CustomSidebar} from "./CustomSidebar"

// The admin theme overrides the drawer's default widths.
const {width: OPEN_WIDTH, closedWidth: CLOSED_WIDTH} = fullAdminTheme.sidebar

interface Scenario {
    /** Whether the sidebar starts expanded. */
    sidebarOpen: boolean
    appBarAlwaysOn: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>

/** The menu the layout puts in the sidebar, reduced to its toggle. */
function Menu() {
    const [open, setOpen] = useSidebarState()
    return (
        <nav aria-label="Main menu">
            <button type="button" onClick={() => setOpen(!open)}>
                {open ? "Collapse" : "Expand"}
            </button>
        </nav>
    )
}

const meta = {
    title: "Admin/Menu/CustomSidebar",
    component: CustomSidebar,
    args: {sidebarOpen: true, appBarAlwaysOn: false},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({sidebarOpen, appBarAlwaysOn}) => (
        <AdminStoryProvider boundary={graphql} store={memoryStore({"sidebar.open": sidebarOpen})}>
            <CustomSidebar appBarAlwaysOn={appBarAlwaysOn}>
                <Menu />
            </CustomSidebar>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const paperWidth = (canvasElement: HTMLElement) => {
    const paper = canvasElement.querySelector(".MuiDrawer-paper")
    if (!paper) throw new Error("Missing sidebar paper")
    return paper.getBoundingClientRect().width
}

export const Expanded: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Collapse"})).toBeVisible()
        await waitFor(() => expect(paperWidth(canvasElement)).toBe(OPEN_WIDTH))
        expect(canvasElement.querySelector(".print-hidden")).not.toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const Collapsed: Story = {
    args: {sidebarOpen: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: "Expand"})).toBeVisible()
        await waitFor(() => expect(paperWidth(canvasElement)).toBe(CLOSED_WIDTH))
        expect(graphql.calls).toEqual([])
    },
}

export const ToggleFromTheMenu: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Collapse"}))
        await waitFor(() => expect(paperWidth(canvasElement)).toBe(CLOSED_WIDTH))
        await userEvent.click(canvas.getByRole("button", {name: "Expand"}))
        await waitFor(() => expect(paperWidth(canvasElement)).toBe(OPEN_WIDTH))
        expect(graphql.calls).toEqual([])
    },
}

export const AppBarAlwaysOn: Story = {
    args: {appBarAlwaysOn: true},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(paperWidth(canvasElement)).toBe(OPEN_WIDTH))
        expect(canvasElement.querySelector(".RaSidebar-appBarCollapsed")).toBeNull()
    },
}
