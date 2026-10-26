// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {SettingsConfigurationSigning} from "./SettingsConfigurationSigning"
import type {ConfigurationSigning} from "./configurationSigning"
import roots from "../../lib/signing/__fixtures__/test-pnpki-chain.pem?raw"

interface Scenario {
    roles: string[]
    failed: boolean
    emptySettings: boolean
}
let graphql: ReturnType<typeof graphqlBoundary>
let settings: {language?: string; configuration_signing?: ConfigurationSigning} | null
const meta = {
    title: "Admin/Settings/ConfigurationPackages",
    component: SettingsConfigurationSigning,
    args: {
        roles: [IPermissions.TENANT_WRITE, IPermissions.SETTINGS_MENU],
        failed: false,
        emptySettings: false,
    },
    beforeEach: async ({args}) => {
        settings = args.emptySettings ? null : {language: "en"}
        graphql = graphqlBoundary(
            {
                GetConfigurationSigning: () => ({
                    data: {sequent_backend_tenant_by_pk: {id: TENANT_ID, settings}},
                }),
                SaveConfigurationSigning: ({variables}) => {
                    if (args.failed) throw new Error("Synthetic refusal")
                    settings = {
                        ...settings,
                        ...(variables.settings as {configuration_signing: ConfigurationSigning}),
                    }
                    return {
                        data: {
                            update_sequent_backend_tenant: {
                                affected_rows: 1,
                                returning: [{id: TENANT_ID, settings}],
                            },
                        },
                    }
                },
                InitializeConfigurationSigning: ({variables}) => {
                    if (args.failed) throw new Error("Synthetic refusal")
                    settings = {
                        ...settings,
                        ...(variables.settings as {configuration_signing: ConfigurationSigning}),
                    }
                    return {
                        data: {
                            update_sequent_backend_tenant: {
                                affected_rows: 1,
                                returning: [{id: TENANT_ID, settings}],
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} roles={roles} auth={{tenantId: TENANT_ID}}>
            <SettingsConfigurationSigning />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function fill(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    for (const name of ["Package-signing CAs", "Election board approval CAs"]) {
        const input = await canvas.findByLabelText(name)
        await userEvent.upload(
            input,
            new File([roots], "organizer-ca.pem", {type: "application/x-pem-file"})
        )
    }
    await waitFor(() => expect(canvas.getAllByRole("checkbox")).toHaveLength(2))
    const save = canvas.getByRole("button", {name: "Save certificate settings"})
    await expect(save).toBeDisabled()
    for (const box of canvas.getAllByRole("checkbox")) await userEvent.click(box)
    await expect(save).toBeEnabled()
    return save
}
export const ReadyToSave: Story = {
    play: async ({canvasElement}) => {
        await fill(canvasElement)
    },
}
export const SaveAndReadBack: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await fill(canvasElement))
        await expect(
            await within(canvasElement).findByText(
                "Certificate settings saved and read back successfully."
            )
        ).toBeVisible()
        expect(settings?.language).toBe("en")
        expect(settings?.configuration_signing?.required_approvals).toBe(2)
        expect(graphql.calls.filter(({name}) => name === "SaveConfigurationSigning")).toHaveLength(
            1
        )
        expect(graphql.calls.filter(({name}) => name === "GetConfigurationSigning")).toHaveLength(2)
    },
}
export const RefusedByServer: Story = {
    args: {failed: true},
    play: async ({canvasElement}) => {
        await userEvent.click(await fill(canvasElement))
        await expect(
            await within(canvasElement).findByText(/Settings were not confirmed/)
        ).toBeVisible()
        expect(settings?.configuration_signing).toBeUndefined()
    },
}
export const WithoutPermission: Story = {
    args: {roles: [IPermissions.SETTINGS_MENU]},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const InitiallyNullSettings: Story = {
    args: {emptySettings: true},
    play: async ({canvasElement}) => {
        await userEvent.click(await fill(canvasElement))
        await expect(
            await within(canvasElement).findByText(
                "Certificate settings saved and read back successfully."
            )
        ).toBeVisible()
        expect(
            graphql.calls.filter(({name}) => name === "InitializeConfigurationSigning")
        ).toHaveLength(1)
    },
}
