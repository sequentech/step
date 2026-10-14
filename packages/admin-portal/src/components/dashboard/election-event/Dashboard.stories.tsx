// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createRef} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    STORY_IDS,
    eventRecord,
    keysCeremonyRecord,
    tallySessionRecord,
} from "@/__stories__/fixtures"
import {
    EStoryPermissions,
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../../ui-essentials/.storybook/globals"
import {pending} from "../../../../../ui-essentials/.storybook/screens"
import DashboardElectionEvent from "./Dashboard"
import {
    ipAddressRows,
    statsDateRange,
    votersByChannel,
    votesPerDay,
} from "../__stories__/DashboardFixture"

interface Scenario {
    /** What the statistics query does. */
    stats: "records" | "loading" | "error"
    /** Whether the event offers kiosk voting. */
    kiosk: boolean
    onMount: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
const refreshRef = createRef<HTMLButtonElement>()

const hasTally = (workflow: EStoryWorkflow) =>
    [EStoryWorkflow.TALLY, EStoryWorkflow.RESULTS].includes(workflow)

function Fixture({kiosk, onMount}: Scenario) {
    const {permissions, workflow} = useStoryGlobals()
    const event = eventRecord(workflow, {
        voting_channels: {online: true, kiosk, early_voting: false, telephone: false},
    })
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <RecordContextProvider value={event}>
                <DashboardElectionEvent refreshRef={refreshRef} onMount={onMount} />
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const progressDefect = {
    expectedFailure: {
        reason: "The dashboard's loading spinner is a progressbar without an accessible name.",
        a11y: ["aria-progressbar-name"],
    },
}

const meta = {
    title: "Admin/Dashboard/Election event/DashboardElectionEvent",
    component: DashboardElectionEvent,
    args: {stats: "records", kiosk: false, onMount: fn()},
    argTypes: {stats: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection of the IP address list labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: async ({args, globals}) => {
        // The voter portal links come from sequent-core.
        await initCore()
        const {workflow} = readStoryGlobals(globals)
        data = resourceBoundary({
            sequent_backend_tally_session: hasTally(workflow) ? [tallySessionRecord(workflow)] : [],
            ip_address: ipAddressRows(),
        })
        graphql = graphqlBoundary(
            {
                GetElectionEventStats: () => {
                    if (args.stats === "loading") return pending()
                    if (args.stats === "error") throw new Error("Synthetic stats unavailable")
                    return {
                        data: {
                            stats: {
                                total_eligible_voters: 1204,
                                total_distinct_voters: 38,
                                total_areas: 2,
                                total_elections: 1,
                                voters_by_channel: votersByChannel,
                                votes_per_day: votesPerDay,
                            },
                            election_event: [
                                {
                                    statistics: {
                                        num_emails_sent: 12,
                                        num_sms_sent: 3,
                                        num_whatsapp_sent: 40,
                                    },
                                },
                            ],
                        },
                    }
                },
                ListKeysCeremony: () => {
                    const ceremony = keysCeremonyRecord(workflow)
                    return {
                        data: {
                            list_keys_ceremony: {
                                items: [{...ceremony, permission_label: null}],
                                total: {aggregate: {count: 1}},
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const STEPS: Record<EStoryWorkflow, string> = {
    [EStoryWorkflow.CREATED]: "Created",
    [EStoryWorkflow.KEYS]: "Keys",
    [EStoryWorkflow.PUBLISHED]: "Publish",
    [EStoryWorkflow.STARTED]: "Started",
    [EStoryWorkflow.ENDED]: "Ended",
    [EStoryWorkflow.TALLY]: "Ended",
    [EStoryWorkflow.RESULTS]: "Results",
}

const statsCalls = () => graphql.calls.filter(({name}) => name === "GetElectionEventStats")

const card = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByText(label).parentElement as HTMLElement

export const Populated: Story = {
    play: async ({canvasElement, globals, args}) => {
        const canvas = within(canvasElement)
        const {permissions, workflow} = readStoryGlobals(globals)
        const eligible = await canvas.findByText("Eligible Voters")
        await expect(within(eligible.parentElement as HTMLElement).getByText("1,204")).toBeVisible()
        await expect(within(card(canvasElement, "Emails sent")).getByText("12")).toBeVisible()
        await waitFor(() =>
            expect(canvasElement.querySelector('[aria-current="step"]')).toHaveTextContent(
                STEPS[workflow]
            )
        )
        expect(statsCalls()[0]).toEqual({
            name: "GetElectionEventStats",
            variables: {
                tenantId: TENANT_ID,
                electionEventId: STORY_IDS.event,
                ...statsDateRange(),
                timeResolution: "day",
                bucketCount: 7,
            },
            headers: {},
        })
        expect(graphql.calls.find(({name}) => name === "ListKeysCeremony")?.headers).toEqual({
            "x-hasura-role":
                permissions === EStoryPermissions.TRUSTEE ? "trustee-ceremony" : "admin-ceremony",
        })
        const login = canvas.getByRole("link", {name: "Voter Login URL"})
        expect(login.getAttribute("href")).toContain("voting.admin-story.invalid")
        expect(canvas.queryByRole("link", {name: "Voter Enroll Kiosk URL"})).toBeNull()
        if (permissions === EStoryPermissions.ADMIN) {
            await expect(await canvas.findByRole("row", {name: /192\.0\.2\.10/})).toBeVisible()
        } else {
            expect(canvas.queryByText("IP Addresses")).not.toBeInTheDocument()
        }
        expect(args.onMount).toHaveBeenCalled()
    },
}

export const Loading: Story = {
    args: {stats: "loading"},
    parameters: progressDefect,
    play: async ({canvasElement}) => {
        await waitFor(() => expect(statsCalls()).toHaveLength(1))
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByText("Eligible Voters")).not.toBeInTheDocument()
    },
}

export const StatsUnavailable: Story = {
    args: {stats: "error"},
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Without statistics every count is unknown, and so are the votes over time.
        const eligible = await canvas.findByText("Eligible Voters")
        await expect(within(eligible.parentElement as HTMLElement).getByText("-")).toBeVisible()
        expect(canvas.queryByRole("progressbar")).toBeNull()
        const votes = canvas.getByText("Votes over time").closest(".MuiPaper-root") as HTMLElement
        await expect(within(votes).getByText("-")).toBeVisible()
        expect(statsCalls()).toHaveLength(1)
    },
}

export const TrusteeWithoutIpAddresses: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("Eligible Voters")
        expect(graphql.calls.find(({name}) => name === "ListKeysCeremony")?.headers).toEqual({
            "x-hasura-role": "trustee-ceremony",
        })
        expect(within(canvasElement).queryByText("IP Addresses")).not.toBeInTheDocument()
        expect(data.calls.map(({args}) => args[0])).not.toContain("ip_address")
    },
}

export const KioskEnrollLink: Story = {
    args: {kiosk: true},
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const link = await within(canvasElement).findByRole("link", {
            name: "Voter Enroll Kiosk URL",
        })
        expect(link.getAttribute("href")).toMatch(
            /^https:\/\/kiosk\.admin-story\.invalid\/.*\?kiosk$/
        )
    },
}

export const ChangeTheTimeResolution: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("combobox", {name: "Time resolution"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Hour"}))
        await waitFor(() =>
            expect(statsCalls().at(-1)?.variables).toMatchObject({
                timeResolution: "hour",
                bucketCount: 24,
            })
        )
        await expect(
            await canvas.findByRole("combobox", {name: "Time resolution"})
        ).toHaveTextContent("Hour")
    },
}

export const RefreshRefetchesTheStats: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("Eligible Voters")
        expect(statsCalls()).toHaveLength(1)
        refreshRef.current?.click()
        await waitFor(() => expect(statsCalls()).toHaveLength(2))
    },
}
