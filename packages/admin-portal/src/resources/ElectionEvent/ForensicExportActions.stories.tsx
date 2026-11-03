// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ListContextProvider, ResourceContextProvider, useList} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {recordDownloads} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {ForensicExportActions} from "./ForensicExportActions"

interface Scenario {
    selection: "selected" | "empty"
}
const rows = [
    {
        id: "address",
        ip: "192.0.2.10",
        country: "ES",
        vote_count: 3,
        pseudonyms: ["7QKM-2HXD"],
        shared: true,
    },
]
let boundary: ReturnType<typeof graphqlBoundary>
let downloads: ReturnType<typeof recordDownloads>

function Fixture({selection}: Scenario) {
    const list = useList({data: rows, filter: {ip: "192.0.2.10"}})
    return (
        <ResourceContextProvider value="ip_address">
            <ListContextProvider
                value={{...list, selectedIds: selection === "selected" ? ["address"] : []}}
            >
                <ForensicExportActions electionEventId={EVENT_ID} selectedOnly />
            </ListContextProvider>
        </ResourceContextProvider>
    )
}

const meta = {
    title: "Admin/Election event/ForensicExportActions",
    component: ForensicExportActions,
    args: {selection: "selected"},
    beforeEach: async () => {
        boundary = graphqlBoundary(
            {
                ForensicNetworkSummary: () => ({
                    data: {forensic_network_summary: {payload: {items: rows, total: rows.length}}},
                }),
            },
            {schema: true}
        )
        await boundary.ready
        downloads = recordDownloads()
        return downloads.restore
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <Fixture {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ExportSelection: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
        await waitFor(() => expect(downloads.downloads).toHaveLength(1))
        expect(downloads.downloads[0].name).toBe("forensic-network.csv")
        expect(boundary.calls).toMatchObject([
            {
                variables: {
                    electionEventId: EVENT_ID,
                    ip: "192.0.2.10",
                    offset: 0,
                    limit: 100,
                    operation: "export",
                },
                headers: {"x-hasura-role": IPermissions.FORENSIC_READ},
            },
        ])
    },
}

export const NoSelection: Story = {
    args: {selection: "empty"},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("button", {name: "Export"})).toBeNull()
        expect(boundary.calls).toHaveLength(0)
    },
}
