// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {Typography} from "@mui/material"
import {initCore} from "@sequentech/ui-core"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {tenantRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {
    TreeStory,
    treeServices,
    type TreeServices,
} from "./menu/items/__stories__/ElectionTreeFixture"
import {CustomLayout} from "./CustomLayout"
import {useStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    sidebarOpen: boolean
}

let services: TreeServices

// The shared header's banner sits in react-admin's app bar banner, and the side
// menu puts the tenant selector and event tree in a menu that only admits items.
const layoutDefects = (...more: string[]) => ({
    reason: "The header's banner is nested in the app bar's, and the side menu holds non-items in a menu.",
    a11y: [
        "aria-required-children",
        "landmark-banner-is-top-level",
        "landmark-no-duplicate-banner",
        "landmark-unique",
        ...more,
    ],
})

function Fixture({sidebarOpen}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <TreeStory
            services={services}
            role={permissions}
            sidebarOpen={sidebarOpen}
            tenant={tenantRecord as Sequent_Backend_Tenant}
            createProvider={false}
        >
            <CustomLayout>
                <Typography variant="h4" component="h1">
                    Election events
                </Typography>
            </CustomLayout>
        </TreeStory>
    )
}

const meta = {
    title: "Admin/Components/CustomLayout",
    component: CustomLayout,
    args: {sidebarOpen: true},
    parameters: {expectedFailure: layoutDefects()},
    beforeEach: async () => {
        await initCore()
        services = treeServices()
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tenantReads = () =>
    services.data.calls.filter(
        ({method, args}) => method === "getOne" && args[0] === "sequent_backend_tenant"
    )

export const Populated: Story = {
    parameters: {widgets: ["SequentSidebar"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Election events"})).toBeVisible()
        await expect(await canvas.findByText("example-council")).toBeVisible()
        await expect(await canvas.findByRole("link", {name: "Council"})).toBeVisible()
        expect(canvas.getByRole("img", {name: "Logo Image"})).toBeVisible()
        // The app bar, the tenant selector and the tenant's look and feel each read the tenant.
        expect(tenantReads()).not.toEqual([])
        expect(tenantReads()).toContainEqual({
            method: "getOne",
            args: ["sequent_backend_tenant", expect.objectContaining({id: TENANT_ID})],
        })
    },
}

export const CreateAnEvent: Story = {
    parameters: {
        widgets: ["SequentSidebar"],
        expectedFailure: {
            reason: "The creation drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("link", {name: "Council"})
        await userEvent.click(canvas.getByRole("button", {name: "Create an Election Event"}))
        const menu = await within(document.body).findByRole("menu")
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Create an Election Event"})
        )
        const name = await within(document.body).findByRole("textbox", {name: "Name"})
        await waitFor(() => expect(name).toBeVisible())
        expect(services.graphql.calls.map(({name}) => name)).not.toContain("CreateElectionEvent")
    },
}

export const ImportAnEvent: Story = {
    parameters: {
        widgets: ["SequentSidebar"],
        expectedFailure: {
            reason: "The import drawer is a modal dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("link", {name: "Council"})
        await userEvent.click(canvas.getByRole("button", {name: "Create an Election Event"}))
        const menu = await within(document.body).findByRole("menu")
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Import Election Event"}))
        const hint = await within(document.body).findByText(
            "Import Election Events using a JSON file."
        )
        await waitFor(() => expect(hint).toBeVisible())
        expect(services.graphql.calls.map(({name}) => name)).not.toContain("ImportElectionEvent")
    },
}

export const CollapsedSidebar: Story = {
    args: {sidebarOpen: false},
    parameters: {
        widgets: ["SequentSidebar"],
        expectedFailure: {
            ...layoutDefects("link-name"),
            reason: "Besides the nested banners and the menu holding non-items, the collapsed menu's entries are icon links without names.",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Election events"})).toBeVisible()
        await waitFor(() => expect(tenantReads()).not.toEqual([]))
        expect(canvas.queryByText("example-council")).toBeNull()
        expect(canvas.queryByRole("link", {name: "Council"})).toBeNull()
    },
}
