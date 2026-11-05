// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsVotingChannels} from "./SettingsVotingChannel"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
}

const Tab = settingsTab(SettingsVotingChannels)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsVotingChannels",
    component: SettingsVotingChannels,
    args: {reads: "records"},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The channel switches are not labelled by the channel name beside them.",
            a11y: ["label"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: [settingsTenant()]}, {reads: args.reads})
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

const channelSwitch = async (canvasElement: HTMLElement, channel: string) => {
    const label = await within(canvasElement).findByText(
        i18n.t(`electionTypeScreen.common.${channel}Voting`)
    )
    return within(label.parentElement as HTMLElement).getByRole("switch")
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(async () => expect(await channelSwitch(canvasElement, "kiosk")).toBeChecked())
        await expect(await channelSwitch(canvasElement, "online")).toBeChecked()
        await expect(await channelSwitch(canvasElement, "telephone")).not.toBeChecked()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TENANT_RESOURCE],
        ])
    },
}

export const EnableTelephoneVoting: Story = {
    play: async ({canvasElement}) => {
        await waitFor(async () => expect(await channelSwitch(canvasElement, "kiosk")).toBeChecked())
        await userEvent.click(await channelSwitch(canvasElement, "telephone"))
        await expect(await channelSwitch(canvasElement, "telephone")).toBeChecked()
        // The change is undoable until its notification closes.
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
                        data: {voting_channels: {online: true, kiosk: true, telephone: true}},
                    }),
                },
            ])
        )
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

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        // Without the tenant, only online voting shows as enabled.
        await expect(await channelSwitch(canvasElement, "online")).toBeChecked()
        await expect(await channelSwitch(canvasElement, "kiosk")).not.toBeChecked()
    },
}
