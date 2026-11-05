// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {memoryStore} from "react-admin"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {type ReadState, resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {groupRoles} from "@/__stories__/storyAuth"
import {TENANT_RESOURCE, tenantRecords} from "@/resources/Tenant/__stories__/TenantFixture"
import SelectTenants from "./SelectTenants"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    /** Signed in to the default tenant, which makes the admin a super admin. */
    superAdmin: boolean
    /** Whether the sidebar is expanded. */
    sidebarOpen: boolean
    roles?: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({superAdmin, sidebarOpen, roles}: Scenario) {
    const {permissions} = useStoryGlobals()
    const granted = roles ?? groupRoles(permissions)
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            // As AuthContextProvider checks it: a super admin passes tenant-less checks.
            auth={{
                isAuthorized: (checkSuperAdmin, someTenantId, permission) =>
                    ((checkSuperAdmin && superAdmin) || someTenantId === TENANT_ID) &&
                    [permission].flat().some((name) => granted.includes(name)),
            }}
            store={memoryStore({"sidebar.open": sidebarOpen})}
        >
            <SelectTenants />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Menu/Items/SelectTenants",
    component: SelectTenants,
    args: {reads: "records", superAdmin: true, sidebarOpen: true},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: tenantRecords()}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const read = () => data.calls.find(({method}) => method === "getOne")?.args

const addTenant = (canvasElement: HTMLElement) =>
    within(canvasElement).queryByRole("button", {name: "Create new tenant"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("example-council")).toBeVisible()
        expect(addTenant(canvasElement)).toBeVisible()
        expect(read()).toEqual([TENANT_RESOURCE, expect.objectContaining({id: TENANT_ID})])
    },
}

export const TenantAdmin: Story = {
    args: {superAdmin: false},
    play: async ({canvasElement}) => {
        // Only a super admin creates tenants, whatever the tenant roles say.
        await expect(await within(canvasElement).findByText("example-council")).toBeVisible()
        expect(addTenant(canvasElement)).toBeNull()
    },
}

export const WithoutTenantCreatePermission: Story = {
    args: {roles: ["tenant-read"]},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("example-council")).toBeVisible()
        expect(addTenant(canvasElement)).toBeNull()
    },
}

export const CollapsedSidebar: Story = {
    args: {sidebarOpen: false},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(read()).toBeDefined())
        expect(within(canvasElement).getByTestId("AccountCircleIcon")).toBeInTheDocument()
        expect(within(canvasElement).queryByText("example-council")).toBeNull()
        expect(addTenant(canvasElement)).toBeNull()
    },
}

export const LoadingTenant: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(read()).toBeDefined())
        expect(within(canvasElement).queryByText("example-council")).toBeNull()
        expect(addTenant(canvasElement)).toBeNull()
    },
}

export const OpenTheNewTenantDrawer: Story = {
    parameters: {
        expectedFailure: {
            reason: "The new tenant drawer is a dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("example-council")
        const button = addTenant(canvasElement)
        if (!button) throw new Error("Missing add tenant button")
        await userEvent.click(button)
        await expect(
            await within(document.body).findByRole("textbox", {name: /Slug/})
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}
