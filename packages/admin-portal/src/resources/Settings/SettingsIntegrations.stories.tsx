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
import {SettingsIntegrations} from "./SettingsIntegrations"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** The signed-in user's roles. */
    roles: string[]
}

const Tab = settingsTab(SettingsIntegrations)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsIntegrations",
    component: SettingsIntegrations,
    args: {reads: "records", roles: [IPermissions.TENANT_WRITE]},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: [settingsTenant()]}, {reads: args.reads})
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
            {/* As the settings screen's tenant resource renders the tab. */}
            <ResourceContextProvider value={TENANT_RESOURCE}>
                <Tab />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const keyInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {
        name: i18n.t("integrationsScreen.common.gapiKey"),
    })
const emailInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {
        name: i18n.t("integrationsScreen.common.gapiEmail"),
    })

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await keyInput(canvasElement)).toBeVisible()
        await expect(await emailInput(canvasElement)).toBeVisible()
        // Nothing to save until a field changes.
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

export const SaveTheCalendarEmail: Story = {
    play: async ({canvasElement}) => {
        await userEvent.type(await emailInput(canvasElement), "events@admin-story.invalid")
        const save = within(canvasElement).getByRole("button", {name: "Save"})
        await waitFor(() => expect(save).toBeEnabled())
        await userEvent.click(save)
        await within(document.body).findByText("Element updated")
        expect(data.writes).toEqual([])
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: TENANT_RESOURCE,
                    params: expect.objectContaining({
                        id: TENANT_ID,
                        data: {
                            settings: expect.objectContaining({
                                gapi_email: "events@admin-story.invalid",
                                // Other settings are kept.
                                voting_countries: ["ES"],
                            }),
                        },
                    }),
                },
            ])
        )
    },
}

export const RejectAMalformedKey: Story = {
    play: async ({canvasElement}) => {
        await userEvent.type(await keyInput(canvasElement), "x")
        const message = await within(document.body).findByText(
            i18n.t("integrationsScreen.errors.invalidGapiKey")
        )
        await waitFor(() => expect(message).toBeVisible())
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
        expect(data.writes).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TENANT_READ]},
    play: async ({canvasElement}) => {
        await userEvent.type(await emailInput(canvasElement), "events@admin-story.invalid")
        // Without the tab's own button the form shows react-admin's default one,
        // which submits to no save handler.
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        expect(within(document.body).queryByText("Element updated")).toBeNull()
        expect(data.writes).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
    },
}
