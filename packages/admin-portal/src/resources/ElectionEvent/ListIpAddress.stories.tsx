// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {recordDownloads} from "@/__stories__/downloads"
import {
    STORY_IDS,
    electionPresentation,
    electionRecord,
    eventRecord,
    storyId,
} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ListIpAddress} from "./ListIpAddress"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    empty: boolean
    /** Whether the list is an election's rather than the event's. */
    scope: "event" | "election"
}

const RESOURCE = "ip_address"

const votes = [
    {ip: "192.0.2.10", country: "ES", vote_count: 3, voters_id: "voter-1, voter-2"},
    {ip: "198.51.100.7", country: null, vote_count: 1, voters_id: "voter-3"},
].map((row, index) => ({
    id: storyId(3, index + 5),
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    election_presentation: electionPresentation("Council election"),
    ...row,
}))

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: ReturnType<typeof recordDownloads>

function Fixture({scope}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <RecordContextProvider value={scope === "event" ? eventRecord() : electionRecord()}>
                <ListIpAddress />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const listDefects = {
    expectedFailure: {
        reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
        a11y: ["aria-prohibited-attr", "label"],
    },
}

const meta = {
    title: "Admin/Election event/ListIpAddress",
    component: ListIpAddress,
    args: {reads: "records", empty: false, scope: "event"},
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        scope: {control: "inline-radio", options: ["event", "election"]},
    },
    parameters: listDefects,
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[RESOURCE]: args.empty ? [] : votes, sequent_backend_election: [electionRecord()]},
            {reads: {[RESOURCE]: args.reads}}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
        downloads = recordDownloads()
        return downloads.restore
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const voteRow = (canvasElement: HTMLElement, ip: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(ip.replaceAll(".", "\\."))})

const listFilter = () =>
    (data.calls.filter(({method}) => method === "getList").at(-1)?.args[1] as {filter?: object})
        ?.filter

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("IP Addresses")).toBeVisible()
        const first = await voteRow(canvasElement, "192.0.2.10")
        await expect(within(first).getByText("ES")).toBeVisible()
        await expect(within(first).getByText("3")).toBeVisible()
        await expect(within(first).getByText("Council")).toBeVisible()
        // A vote without a known country shows a dash; voter IDs are hidden by default.
        await expect(
            within(await voteRow(canvasElement, "198.51.100.7")).getByText("-")
        ).toBeVisible()
        expect(canvas.queryByText("voter-3")).toBeNull()
        expect(listFilter()).toEqual({tenant_id: TENANT_ID, election_event_id: EVENT_ID})
    },
}

export const ElectionVotes: Story = {
    args: {scope: "election"},
    play: async ({canvasElement}) => {
        await voteRow(canvasElement, "192.0.2.10")
        expect(listFilter()).toEqual({
            tenant_id: TENANT_ID,
            election_event_id: EVENT_ID,
            election_id: STORY_IDS.election,
        })
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {expectedFailure: null, widgets: ["Empty"]},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No votes yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("table")).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({args}) => args[0])).toContain(RESOURCE))
        expect(within(canvasElement).queryByRole("row", {name: /192\.0\.2\.10/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /192\.0\.2\.10/})).toBeNull()
    },
}

export const FilterByIp: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await voteRow(canvasElement, "192.0.2.10")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "IP"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "IP"}), "198.51.100.7")
        await waitFor(() =>
            expect(listFilter()).toEqual({
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                ip: "198.51.100.7",
            })
        )
        await waitFor(() => expect(canvas.queryByRole("row", {name: /192\.0\.2\.10/})).toBeNull())
        await expect(await voteRow(canvasElement, "198.51.100.7")).toBeVisible()
    },
}

export const ExportCsv: Story = {
    play: async ({canvasElement}) => {
        await voteRow(canvasElement, "192.0.2.10")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
        await waitFor(() => expect(downloads.downloads).toHaveLength(1))
        expect(downloads.downloads[0].name).toBe(`${RESOURCE}.csv`)
        expect(data.calls.filter(({method}) => method === "getList")).toHaveLength(2)
    },
}
