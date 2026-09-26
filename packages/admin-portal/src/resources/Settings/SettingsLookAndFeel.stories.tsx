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
import {SettingsLookAndFeel} from "./SettingsLookAndFeel"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** The signed-in user's roles. */
    roles: string[]
}

const Tab = settingsTab(SettingsLookAndFeel)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsLookAndFeel",
    component: SettingsLookAndFeel,
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

const input = (canvasElement: HTMLElement, field: string) =>
    within(canvasElement).findByRole("textbox", {name: i18n.t(`lookAndFeelScreen.common.${field}`)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await input(canvasElement, "logoUrl")).toBeVisible()
        await expect(await input(canvasElement, "css")).toBeVisible()
        await expect(await input(canvasElement, "helpLinks")).toHaveValue("[]")
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

export const SaveTheLogo: Story = {
    play: async ({canvasElement}) => {
        const logo = "https://assets.admin-story.invalid/council.svg"
        await userEvent.type(await input(canvasElement, "logoUrl"), logo)
        await userEvent.tab()
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
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
                            annotations: expect.objectContaining({logo_url: logo}),
                            settings: expect.objectContaining({
                                help_links: [],
                                voting_countries: ["ES"],
                            }),
                        },
                    }),
                },
            ])
        )
        // Saved changes disable the button until the next edit.
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const RejectMalformedHelpLinks: Story = {
    play: async ({canvasElement}) => {
        const helpLinks = await input(canvasElement, "helpLinks")
        await userEvent.clear(helpLinks)
        await userEvent.type(helpLinks, "not a list")
        await userEvent.tab()
        const message = await within(document.body).findByText(
            i18n.t("lookAndFeelScreen.errors.invalidHelpLinks")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(data.writes).toEqual([])
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TENANT_READ]},
    play: async ({canvasElement}) => {
        await userEvent.type(await input(canvasElement, "css"), "body {{}")
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
