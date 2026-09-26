// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, ResourceContextProvider} from "react-admin"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {EditElectionEventIvr} from "./EditElectionEventIvr"
import {IVR_PHONE, ivrEvent, jsonEditorDefects} from "./__stories__/IvrFixture"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Replaces the signed-in group's roles. */
    roles?: string[]
}

// No emulator is deployed next to Storybook: the tab's loader finds nothing at
// this local address, as in an environment without the emulator.
const IVR_EMULATOR_BASE_URL = "/ivr-story/ivr_emulator_wasm"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

function Fixture({roles}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
            tenant={tenant}
            settings={{IVR_EMULATOR_BASE_URL}}
        >
            <ResourceContextProvider value="sequent_backend_election_event">
                <RecordContextProvider value={ivrEvent()}>
                    <EditElectionEventIvr />
                </RecordContextProvider>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

// Each tab's widget has its own section; these stories show which tab renders it.
const meta = {
    title: "Admin/Election event/EditElectionEventIvr",
    component: EditElectionEventIvr,
    args: {},
    parameters: {...jsonEditorDefects, widgets: ["ConfigTab"]},
    beforeEach: async () => {
        data = resourceBoundary({
            sequent_backend_election_event: [ivrEvent()],
            sequent_backend_phone_blacklist: [],
        })
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tabNames = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("tab")
        .map((tab) => tab.textContent)

const openTab = async (canvasElement: HTMLElement, name: string) => {
    const tab = await within(canvasElement).findByRole("tab", {name})
    await userEvent.click(tab)
    await waitFor(() => expect(tab).toHaveAttribute("aria-selected", "true"))
}

export const Configuration: Story = {
    parameters: {widgets: ["ConfigTab"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("textbox", {name: "Configured phone number"})
        ).toHaveValue(IVR_PHONE)
        expect(tabNames(canvasElement)).toEqual([
            "Configuration",
            "Prompts",
            "Blocklist",
            "Emulator",
        ])
        expect(data.calls).toEqual([])
    },
}

export const Prompts: Story = {
    parameters: {
        expectedFailure: {
            reason:
                "The prompt row actions are unnamed icon buttons, and json-edit-react's default " +
                "theme renders item counts and string values with insufficient contrast.",
            a11y: ["button-name", "color-contrast"],
        },
        widgets: ["PromptsTab"],
    },
    play: async ({canvasElement}) => {
        await openTab(canvasElement, "Prompts")
        await expect(
            await within(canvasElement).findByRole("row", {name: /welcome_prompt/})
        ).toBeVisible()
        expect(
            within(canvasElement).queryByRole("textbox", {name: "Configured phone number"})
        ).toBeNull()
    },
}

export const Blocklist: Story = {
    parameters: {expectedFailure: null, widgets: ["BlacklistTab"]},
    play: async ({canvasElement}) => {
        await openTab(canvasElement, "Blocklist")
        await expect(
            await within(canvasElement).findByText("There are no entries in the blocklist")
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => `${method} ${String(args[0])}`)).toEqual([
            "getList sequent_backend_phone_blacklist",
        ])
    },
}

export const WithoutBlocklistPermission: Story = {
    args: {roles: [IPermissions.ELECTION_EVENT_IVR_TAB]},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("textbox", {name: "Configured phone number"})
        expect(tabNames(canvasElement)).toEqual(["Configuration", "Prompts", "Emulator"])
    },
}

export const Emulator: Story = {
    parameters: {expectedFailure: null, widgets: ["EmulatorTab"]},
    play: async ({canvasElement}) => {
        await openTab(canvasElement, "Emulator")
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Loading the emulator system")).toBeVisible()
        await expect(
            await canvas.findByText("The emulator system is not available in your environment")
        ).toBeVisible()
    },
}
