// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect} from "react"
import {Button} from "@mui/material"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"
import {ImportDataDrawer} from "./ImportDataDrawer"
import {
    createFlow,
    CreateFlowStory,
    type CreateFlow,
} from "../create/__stories__/CreateElectionEventFixture"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"
let flow: CreateFlow
export const getEvidence = () => ({
    operations: flow.graphql.calls,
    uploads: flow.uploads.calls,
    unexpected: flow.graphql.unexpected,
    created: flow.created.mock.calls,
})
function DrawerFixture({enabled, converterBaseUrl}: {enabled: boolean; converterBaseUrl?: string}) {
    const settings = useContext(SettingsContext)
    const {openImportDrawer} = useCreateElectionEventStore()
    useEffect(() => openImportDrawer(), [])
    return (
        <SettingsContext.Provider
            value={{
                ...settings,
                globalSettings: {
                    ...settings.globalSettings,
                    ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL: enabled
                        ? (converterBaseUrl ?? "/wasm/election_event_import_converter")
                        : undefined,
                },
            }}
        >
            <Button onClick={openImportDrawer}>Open Import Event</Button>
            <ImportDataDrawer
                title="electionEventScreen.import.eetitle"
                subtitle="electionEventScreen.import.eesubtitle"
                paragraph="electionEventScreen.import.electionEventParagraph"
            />
        </SettingsContext.Provider>
    )
}
type Scenario = {enabled: boolean; importError?: string; converterBaseUrl?: string}
function Fixture(args: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <DrawerFixture {...args} />
        </CreateFlowStory>
    )
}
const meta = {
    title: "Screens/Admin/SimpleHistoricalDrawer",
    component: ImportDataDrawer,
    excludeStories: ["getEvidence"],
    args: {enabled: true},
    beforeEach: async ({args}) => {
        flow = createFlow({importError: args.importError})
        await flow.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const LocalDropZone: Story = {
    play: async () => {
        const view = within(document.body)
        await view.findByText("Import Election Event")
        const drawer = view.getByRole("dialog", {name: "Import Election Event"})
        await expect(drawer.getBoundingClientRect().width).toBe(Math.min(window.innerWidth, 640))
        await expect(document.querySelectorAll('input[type="file"]')).toHaveLength(1)
        await expect(view.getByTestId("drop-input-file")).toHaveAttribute("multiple")
        await expect(view.queryByRole("combobox")).not.toBeInTheDocument()
        await expect(view.queryByText(/SQL|XLSX|converter|historical/i)).not.toBeInTheDocument()
        await expect(
            view.queryByRole("button", {name: "Converted ZIP options"})
        ).not.toBeInTheDocument()
        await expect(flow.graphql.calls).toEqual([])
    },
}
export const PluginDisabled: Story = {
    args: {enabled: false},
    play: async () => {
        const view = within(document.body)
        await view.findByRole("dialog", {name: "Import Election Event"})
        await expect(view.getByTestId("drop-input-file")).not.toHaveAttribute("multiple")
        await expect(view.queryByText(/SQL|XLSX|converter|historical/i)).not.toBeInTheDocument()
        await expect(
            view.queryByRole("button", {name: "Converted ZIP options"})
        ).not.toBeInTheDocument()
        await expect(flow.graphql.calls).toEqual([])
    },
}
export const BlankConfiguration: Story = {
    args: {enabled: true, converterBaseUrl: "   "},
    play: PluginDisabled.play,
}

export const BackendRejects: Story = {
    args: {importError: "Synthetic backend validation refused the package."},
}
