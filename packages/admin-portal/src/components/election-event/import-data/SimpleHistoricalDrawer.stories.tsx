// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect} from "react"
import {Button} from "@mui/material"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
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
function DrawerFixture({enabled}: {enabled: boolean}) {
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
                        ? "/wasm/election_event_import_converter"
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
function Fixture(args: {enabled: boolean; importError?: string}) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <CreateFlowStory flow={flow} role={permissions} tenant={tenant}>
            <DrawerFixture {...args} />
        </CreateFlowStory>
    )
}
const meta = {
    title: "Admin/Election event/Import data/SimpleHistoricalDrawer",
    component: Fixture,
    excludeStories: ["getEvidence"],
    args: {enabled: true},
    beforeEach: async ({args}) => {
        flow = createFlow({importError: args.importError})
        await flow.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies Meta<typeof Fixture>
export default meta
type Story = StoryObj<typeof meta>
export const LocalDropZone: Story = {
    play: async () => {
        const view = within(document.body)
        await view.findByText("Import Election Event")
        const drawer = view.getByRole("dialog", {name: "Import Election Event"})
        await expect(drawer.getBoundingClientRect().width).toBe(Math.min(window.innerWidth, 640))
        await expect(document.querySelectorAll('input[type="file"]')).toHaveLength(1)
        await expect(view.getByTestId("drop-input-file")).toHaveAttribute("multiple")
        await expect(view.queryByRole("combobox")).not.toBeInTheDocument()
        await expect(flow.graphql.calls).toEqual([])
    },
}
export const PluginDisabled: Story = {args: {enabled: false}}

export const BackendRejects: Story = {
    args: {importError: "Synthetic backend validation refused the package."},
}
