// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {atViewport} from "../../../../test-support/storybook/viewport"
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ENetworkDataPolicy, EBallotIdentity} from "@sequentech/ui-core"
import {EventTimeZoneProvider} from "@/providers/EventTimeZoneProvider"
import {IPermissions} from "@/types/keycloak"
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
    networkPolicy?: ENetworkDataPolicy
    allowed?: boolean
    detailPages?: "paged"
}

const RESOURCE = "ip_address"

const votes = [
    {
        ip: "192.0.2.10",
        country: "ES",
        vote_count: 3,
        pseudonyms: ["7QKM-2HXD", "3BCF-5GHJ"],
        shared: true,
    },
    {ip: "198.51.100.7", country: null, vote_count: 1, pseudonyms: ["9KMP-2RST"], shared: false},
    {
        ip: "2001:db8:1234:5678:90ab:cdef:1234:5678",
        country: "FR",
        vote_count: 1204,
        pseudonyms: ["3BCF-5GHJ", "9KMP-2RST", "7QKM-2HXD"],
        pseudonym_count: 18,
        shared: true,
    },
].map((row, index) => ({
    id: storyId(3, index + 5),
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    election_presentation: electionPresentation("Council election"),
    first_ballot: "2028-05-08T03:00:00Z",
    last_ballot: "2028-05-08T04:00:00Z",
    ...row,
}))

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: ReturnType<typeof recordDownloads>

function Fixture({
    scope,
    networkPolicy = ENetworkDataPolicy.FORENSIC_STORE,
    allowed = true,
}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const event = eventRecord(undefined, {
        presentation: {
            ...eventRecord().presentation,
            ballot_identity: EBallotIdentity.PSEUDONYM,
            network_data_policy: networkPolicy,
        },
    })
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={
                allowed
                    ? [IPermissions.ADMIN_USER, IPermissions.FORENSIC_READ]
                    : [IPermissions.ADMIN_USER]
            }
            tenant={tenant}
        >
            <EventTimeZoneProvider event={event}>
                <RecordContextProvider value={scope === "event" ? event : electionRecord()}>
                    <ListIpAddress />
                </RecordContextProvider>
            </EventTimeZoneProvider>
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
        graphql = graphqlBoundary(
            {
                ForensicNetworkSummary: () => ({
                    data: {
                        forensic_network_summary: {payload: {items: votes, total: votes.length}},
                    },
                }),
                ForensicNetworkDetail: ({variables}) => ({
                    data: {
                        forensic_network_detail: {
                            payload: {
                                ip: "192.0.2.10",
                                country: "ES",
                                vote_count: 3,
                                first_ballot: "2028-05-08T03:00:00Z",
                                last_ballot: "2028-05-08T04:00:00Z",
                                ballots: [
                                    {
                                        ballot_id: variables.cursor ? "2BCD-3EFG" : "FTBE-MHRX",
                                        pseudonym: "7QKM-2HXD",
                                        cast_at: "2028-05-08T03:00:00Z",
                                    },
                                ],
                                next_cursor:
                                    args.detailPages === "paged" && !variables.cursor
                                        ? "next-page"
                                        : null,
                            },
                        },
                    },
                }),
            },
            {schema: true}
        )
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
    ...atViewport(1440),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("region", {name: "IP Addresses"})).toBeVisible()
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
        const first = await voteRow(canvasElement, "192.0.2.10")
        await expect(within(first).getByText(/^ES/)).toBeVisible()
        await expect(within(first).getByText("7QKM-2HXD")).toBeVisible()
        // A ballot without a known country shows a dash; voter identities are absent.
        await expect(
            within(await voteRow(canvasElement, "198.51.100.7")).getByText("—", {exact: false})
        ).toBeVisible()
        const ipv6 = await voteRow(canvasElement, "2001:db8:1234:5678:90ab:cdef:1234:5678")
        expect(ipv6.scrollWidth).toBeLessThanOrEqual(ipv6.clientWidth + 1)
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
        await expect(
            await within(canvasElement).findByText("No ballot network records yet.")
        ).toBeVisible()
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
        expect(downloads.downloads[0].name).toBe("forensic-network.csv")
        expect(data.calls.filter(({method}) => method === "getList")).toHaveLength(1)
        expect(graphql.calls.filter(({name}) => name === "ForensicNetworkSummary")).toMatchObject([
            {
                variables: {electionEventId: EVENT_ID, limit: 100, offset: 0, operation: "export"},
                headers: {"x-hasura-role": IPermissions.FORENSIC_READ},
            },
        ])
    },
}

export const NoNetworkStorage: Story = {
    args: {networkPolicy: ENetworkDataPolicy.DO_NOT_STORE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByText("IP Addresses")).toBeNull()
        expect(data.calls.some(({args}) => args[0] === RESOURCE)).toBe(false)
    },
}

export const WithoutForensicPermission: Story = {
    args: {allowed: false},
    parameters: {expectedFailure: null},
    play: NoNetworkStorage.play,
}

export const AddressDetails: Story = {
    ...atViewport(1440),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "192.0.2.10"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "IP address 192.0.2.10",
        })
        await expect(within(dialog).getByText(/Opening this address was written/)).toBeVisible()
        await expect(await within(dialog).findByText(/FTBE-MHRX/)).toBeVisible()
        expect(dialog.scrollWidth).toBeLessThanOrEqual(dialog.clientWidth + 1)
        await userEvent.click(within(dialog).getByRole("button", {name: "Close"}))
    },
}

export const FilteredElectionDetails: Story = {
    ...atViewport(1440),
    parameters: {...atViewport(1440).parameters, expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await voteRow(canvasElement, "192.0.2.10")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Election Name"})
        )
        const election = await canvas.findByRole("combobox", {name: /Election Name/})
        await userEvent.type(election, "Council")
        await userEvent.click(await within(document.body).findByRole("option", {name: "Council"}))
        await waitFor(() => expect(listFilter()).toMatchObject({election_id: STORY_IDS.election}))
        await userEvent.click(await canvas.findByRole("button", {name: "192.0.2.10"}))
        await within(document.body).findByRole("dialog", {name: "IP address 192.0.2.10"})
        await waitFor(() =>
            expect(
                graphql.calls.filter(({name}) => name === "ForensicNetworkDetail")
            ).toMatchObject([
                {
                    variables: {
                        electionId: STORY_IDS.election,
                        electionEventId: EVENT_ID,
                        ip: "192.0.2.10",
                    },
                },
            ])
        )
    },
}

export const PopulatedPhone: Story = {
    ...Populated,
    ...atViewport(390),
    parameters: {...Populated.parameters, ...atViewport(390).parameters},
}

export const PopulatedTablet: Story = {
    ...Populated,
    ...atViewport(820),
    parameters: {...Populated.parameters, ...atViewport(820).parameters},
}

export const FilterByIpPhone: Story = {
    ...FilterByIp,
    ...atViewport(390),
    parameters: {...FilterByIp.parameters, ...atViewport(390).parameters},
}

export const FilterByIpTablet: Story = {
    ...FilterByIp,
    ...atViewport(820),
    parameters: {...FilterByIp.parameters, ...atViewport(820).parameters},
}

export const ExportCsvPhone: Story = {
    ...ExportCsv,
    ...atViewport(390),
    parameters: {...ExportCsv.parameters, ...atViewport(390).parameters},
}

export const ExportCsvTablet: Story = {
    ...ExportCsv,
    ...atViewport(820),
    parameters: {...ExportCsv.parameters, ...atViewport(820).parameters},
}

export const AddressDetailsPhone: Story = {
    ...AddressDetails,
    ...atViewport(390),
    parameters: {...AddressDetails.parameters, ...atViewport(390).parameters},
}

export const AddressDetailsTablet: Story = {
    ...AddressDetails,
    ...atViewport(820),
    parameters: {...AddressDetails.parameters, ...atViewport(820).parameters},
}

export const NoNetworkStoragePhone: Story = {
    ...NoNetworkStorage,
    ...atViewport(390),
    parameters: {...NoNetworkStorage.parameters, ...atViewport(390).parameters},
}

export const NoNetworkStorageTablet: Story = {
    ...NoNetworkStorage,
    ...atViewport(820),
    parameters: {...NoNetworkStorage.parameters, ...atViewport(820).parameters},
}

export const PaginatedAddressDetails: Story = {
    parameters: {expectedFailure: null},
    args: {detailPages: "paged"},
    play: async ({canvasElement}) => {
        await userEvent.click(
            await within(canvasElement).findByRole("button", {name: "192.0.2.10"})
        )
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "IP address 192.0.2.10"})
        )
        await expect(await dialog.findByRole("cell", {name: "FTBE-MHRX"})).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Next"}))
        await expect(await dialog.findByRole("cell", {name: "2BCD-3EFG"})).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Previous"}))
        await expect(await dialog.findByRole("cell", {name: "FTBE-MHRX"})).toBeVisible()
        expect(
            graphql.calls
                .filter(({name}) => name === "ForensicNetworkDetail")
                .map(({variables}) => variables.cursor)
        ).toEqual([null, "next-page", null])
        expect(dialog.queryByRole("button", {name: "Try again"})).toBeNull()
    },
}
