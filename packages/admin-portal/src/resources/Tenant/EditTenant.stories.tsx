// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {Route, Routes} from "react-router"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {EditTenant} from "./EditTenant"
import {ListTenant} from "./ListTenant"
import {TENANT_RESOURCE, tenantRecords} from "./__stories__/TenantFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** Whether saving the tenant fails. */
    failure: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Tenant/EditTenant",
    component: EditTenant,
    args: {reads: "records", failure: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        widgets: ["TenantForm"],
        router: {initialEntries: [`/${TENANT_RESOURCE}/${TENANT_ID}`]},
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TENANT_RESOURCE]: tenantRecords()},
            {
                reads: args.reads,
                writeError: args.failure ? "Synthetic tenant service failure" : undefined,
            }
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    // As the tenant resource routes it: the list, and the list beside the edited tenant.
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceContextProvider value={TENANT_RESOURCE}>
                <Routes>
                    <Route path={`/${TENANT_RESOURCE}`} element={<ListTenant />} />
                    <Route path={`/${TENANT_RESOURCE}/:id`} element={<EditTenant />} />
                </Routes>
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Current location"})

async function slugInput(canvasElement: HTMLElement) {
    const input = await within(canvasElement).findByRole("textbox", {name: "Slug"})
    await waitFor(() => expect(input).toHaveValue("example-council"))
    return input
}

async function saveSlug(canvasElement: HTMLElement, slug: string) {
    const input = await slugInput(canvasElement)
    await userEvent.clear(input)
    await userEvent.type(input, slug)
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox; the JSON inputs' item counts are light grey below the contrast minimum.",
            a11y: ["aria-prohibited-attr", "color-contrast", "label"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await slugInput(canvasElement)
        await expect(canvas.getByText("Customer configuration")).toBeVisible()
        await expect(canvas.getByText(TENANT_ID)).toBeVisible()
        await expect(canvas.getByRole("switch", {name: "Is active"})).toBeChecked()
        // The tenant list stays beside the form.
        await expect(canvas.getByRole("row", {name: /archived-council/})).toBeVisible()
        expect(data.calls.find(({method}) => method === "getOne")?.args).toEqual([
            TENANT_RESOURCE,
            expect.objectContaining({id: TENANT_ID}),
        ])
    },
}

export const SaveTheTenant: Story = {
    play: async ({canvasElement}) => {
        await saveSlug(canvasElement, "renamed-council")
        // Saving returns to the list and can be undone until its notification closes.
        await waitFor(() =>
            expect(location(canvasElement)).toHaveTextContent(`/${TENANT_RESOURCE}`)
        )
        const notification = await within(document.body).findByText("Element updated")
        expect(data.writes).toEqual([])
        await waitFor(() => expect(notification).toBeVisible())
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: TENANT_RESOURCE,
                    params: expect.objectContaining({
                        id: TENANT_ID,
                        data: expect.objectContaining({slug: "renamed-council", is_active: true}),
                    }),
                },
            ])
        )
        await expect(
            await within(canvasElement).findByRole("row", {name: /renamed-council/})
        ).toBeVisible()
    },
}

export const UndoTheChange: Story = {
    play: async ({canvasElement}) => {
        await saveSlug(canvasElement, "renamed-council")
        const body = within(document.body)
        await userEvent.click(await body.findByRole("button", {name: "Undo"}))
        await waitFor(() => expect(body.queryByText("Element updated")).toBeNull())
        expect(data.writes).toEqual([])
        await expect(
            await within(canvasElement).findByRole("row", {name: /example-council/})
        ).toBeVisible()
    },
}

export const SaveFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await saveSlug(canvasElement, "renamed-council")
        await within(document.body).findByText("Element updated")
        await userEvent.keyboard("{Escape}")
        const message = await within(document.body).findByText("Synthetic tenant service failure")
        await waitFor(() => expect(message).toBeVisible())
        expect(data.writes.map(({method}) => method)).toEqual(["update"])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("textbox", {name: "Slug"})).toBeNull()
    },
}
