// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsTemplates} from "./SettingsTemplates"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** Whether the tenant sends SMS messages. */
    sms: boolean
}

const Tab = settingsTab(SettingsTemplates)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsTemplates",
    component: SettingsTemplates,
    args: {reads: "records", sms: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The method switches are not labelled by the method name beside them.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[TENANT_RESOURCE]: [settingsTenant({mail: true, sms: args.sms})]},
            {reads: args.reads}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <Tab />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const methodSwitch = async (canvasElement: HTMLElement, method: string) => {
    const label = await within(canvasElement).findByText(
        i18n.t(`electionTypeScreen.common.${method}`)
    )
    return within(label.parentElement as HTMLElement).getByRole("switch")
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await methodSwitch(canvasElement, "mail")).toBeChecked()
        await expect(await methodSwitch(canvasElement, "sms")).not.toBeChecked()
        // The methods are shown but cannot be changed here.
        await expect(await methodSwitch(canvasElement, "mail")).toBeDisabled()
        await expect(await methodSwitch(canvasElement, "sms")).toBeDisabled()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

export const SmsEnabled: Story = {
    args: {sms: true},
    play: async ({canvasElement}) => {
        await waitFor(async () => expect(await methodSwitch(canvasElement, "sms")).toBeChecked())
        expect(data.writes).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("switch")).toBeNull()
    },
}
