// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ListTenant} from "./ListTenant"
import {ARCHIVED_TENANT_ID, TENANT_RESOURCE, tenantRecords} from "./__stories__/TenantFixture"

interface Scenario {
    /** What reading the tenants does. */
    reads: ReadState
    /** Whether there are no tenants. */
    empty: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Tenant/ListTenant",
    component: ListTenant,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TENANT_RESOURCE]: args.empty ? [] : tenantRecords()},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceContextProvider value={TENANT_RESOURCE}>
                <ListTenant />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tenantRow = (canvasElement: HTMLElement, slug: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(slug)})

const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Current location"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("heading", {name: "Tenants"})).toBeVisible()
        const active = await tenantRow(canvasElement, "example-council")
        await expect(within(active).getByTestId("true")).toBeInTheDocument()
        const archived = await tenantRow(canvasElement, "archived-council")
        await expect(within(archived).getByTestId("false")).toBeInTheDocument()
        // The ID column is hidden until the user selects it.
        expect(canvas.queryByText(TENANT_ID)).not.toBeInTheDocument()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByRole("row", {name: /example-council/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /example-council/})).toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is light grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("No Sequent backend tenants yet.")
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const FilterBySlug: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await tenantRow(canvasElement, "archived-council")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Slug"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "Slug"}), "archived-council")
        await waitFor(() => expect(canvas.queryByRole("row", {name: /example-council/})).toBeNull())
        expect(data.calls.at(-1)?.args[1]).toMatchObject({filter: {slug: "archived-council"}})
        await expect(await tenantRow(canvasElement, "archived-council")).toBeVisible()
    },
}

export const OpenATenant: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await tenantRow(canvasElement, "archived-council"))
        await waitFor(() =>
            expect(location(canvasElement)).toHaveTextContent(
                `/${TENANT_RESOURCE}/${ARCHIVED_TENANT_ID}`
            )
        )
    },
}
