// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {SettingsElectionsTypes} from "./SettingsElectionsTypes"
import {
    ELECTION_TYPE_RESOURCE,
    REFERENDUM_TYPE_ID,
    TENANT_RESOURCE,
    electionTypeRecords,
    settingsTab,
    settingsTenant,
} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the election types does. */
    reads: ReadState
    /** Whether the tenant has election types. */
    populated: boolean
    /** The signed-in user's roles. */
    roles: string[]
}

const Tab = settingsTab(SettingsElectionsTypes)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsElectionsTypes",
    component: SettingsElectionsTypes,
    args: {reads: "records", populated: true, roles: [IPermissions.TENANT_WRITE]},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The grid's row checkboxes are unlabelled, its actions column has an empty header, and the rows' edit and delete actions are unnamed icon buttons.",
            a11y: ["aria-prohibited-attr", "button-name", "empty-table-header", "label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                [ELECTION_TYPE_RESOURCE]: args.populated ? electionTypeRecords() : [],
                [TENANT_RESOURCE]: [settingsTenant()],
            },
            {reads: {[ELECTION_TYPE_RESOURCE]: args.reads}}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            roles={roles}
            auth={{tenantId: TENANT_ID}}
        >
            <ResourceContextProvider value={ELECTION_TYPE_RESOURCE}>
                <Tab />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const typeRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

/** The topmost open drawer. */
async function drawer() {
    const drawers = await within(document.body).findAllByRole("presentation")
    const element = drawers[drawers.length - 1]
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await typeRow(canvasElement, "General election")).toBeVisible()
        await expect(await typeRow(canvasElement, "Referendum")).toBeVisible()
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getList", ELECTION_TYPE_RESOURCE],
        ])
        expect(data.writes).toEqual([])
    },
}

export const Empty: Story = {
    args: {populated: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("electionTypeScreen.common.emptyHeader"))
        ).toBeVisible()
        await expect(
            canvas.getByRole("button", {name: i18n.t("electionTypeScreen.common.createNew")})
        ).toBeVisible()
    },
}

export const WithoutTenantWritePermission: Story = {
    args: {roles: [IPermissions.TENANT_READ]},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText(i18n.t("electionTypeScreen.common.emptyHeader"))
        ).toBeVisible()
        expect(canvas.queryByRole("button")).toBeNull()
        // The types are not even listed.
        expect(data.calls).toEqual([])
    },
}

export const CreateAType: Story = {
    play: async ({canvasElement}) => {
        await typeRow(canvasElement, "Referendum")
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.add")})
        )
        const form = await drawer()
        await expect(form.getByText(i18n.t("electionTypeScreen.create.title"))).toBeVisible()
        await userEvent.type(form.getByRole("textbox", {name: "Name"}), "By-election")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "create",
                    resource: ELECTION_TYPE_RESOURCE,
                    params: {data: expect.objectContaining({name: "By-election"})},
                },
            ])
        )
        await expect(await typeRow(canvasElement, "By-election")).toBeVisible()
        await waitFor(() => expect(within(document.body).queryByRole("presentation")).toBeNull())
    },
}

export const RenameAType: Story = {
    play: async ({canvasElement}) => {
        const [edit] = within(await typeRow(canvasElement, "Referendum")).getAllByRole("button")
        await userEvent.click(edit)
        const form = await drawer()
        await expect(await form.findByText(i18n.t("electionTypeScreen.edit.title"))).toBeVisible()
        const name = form.getByRole("textbox", {name: "Name"})
        await waitFor(() => expect(name).toHaveValue("Referendum"))
        await userEvent.clear(name)
        await userEvent.type(name, "Citizens' initiative")
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: ELECTION_TYPE_RESOURCE,
                    params: expect.objectContaining({
                        id: REFERENDUM_TYPE_ID,
                        data: expect.objectContaining({name: "Citizens' initiative"}),
                    }),
                },
            ])
        )
        await expect(await typeRow(canvasElement, "Citizens' initiative")).toBeVisible()
    },
}

export const DeleteAType: Story = {
    play: async ({canvasElement}) => {
        const buttons = within(await typeRow(canvasElement, "Referendum")).getAllByRole("button")
        await userEvent.click(buttons[buttons.length - 1])
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByText(i18n.t("common.message.delete"))).toBeVisible()
        expect(data.writes).toEqual([])
        await userEvent.click(dialog.getByRole("button", {name: i18n.t("common.label.delete")}))
        await waitFor(() =>
            expect(
                data.writes.map(({method, resource, params}) => [method, resource, params.id])
            ).toEqual([["delete", ELECTION_TYPE_RESOURCE, REFERENDUM_TYPE_ID]])
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /Referendum/})).toBeNull()
        )
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getList"))
        expect(within(canvasElement).queryByRole("row", {name: /Referendum/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Referendum/})).toBeNull()
    },
}
