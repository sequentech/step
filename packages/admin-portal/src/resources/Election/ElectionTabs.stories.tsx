// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider, ShowBase} from "react-admin"
import {EElectionEventLockedDown, i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, electionRecord, eventPresentation, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {ElectionTabs} from "./ElectionTabs"
import {
    answerOrPending,
    readsOf,
    recordsOrPending,
} from "../ElectionEvent/__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election"

interface Scenario {
    /** Whether the election has not loaded yet. */
    loading: boolean
    /** Whether the election's event is locked down. */
    eventLockedDown?: boolean
    /** Replaces the signed-in group's roles. */
    roles?: string[]
    /** The election's permission label. */
    label?: string
    /** Permission labels of the signed-in user. */
    userLabels?: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture({roles, userLabels}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
            auth={userLabels ? {permissionLabels: userLabels} : undefined}
            tenant={tenant}
        >
            {/* As in the election's route, whose cached record the data tab's form reuses. */}
            <ResourceContextProvider value={RESOURCE}>
                <ShowBase>
                    <ElectionTabs />
                </ShowBase>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

/** The tabs load their widgets lazily, which a cold dev server serves slowly. */
const LAZY_LOAD_MS = 10_000

// Every tab's widget has its own section: its reads stay loading here, and
// each story asserts that the selected tab mounts that widget.
const meta = {
    title: "Admin/Election/ElectionTabs",
    component: ElectionTabs,
    args: {loading: false},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        router: {
            path: `/${RESOURCE}/:id/*`,
            initialEntries: [`/${RESOURCE}/${STORY_IDS.election}`],
        },
        widgets: ["DashboardTab"],
    },
    beforeEach: async ({args}) => {
        const event = eventRecord(undefined, {
            presentation: {
                ...eventPresentation,
                locked_down: args.eventLockedDown
                    ? EElectionEventLockedDown.LOCKED_DOWN
                    : EElectionEventLockedDown.NOT_LOCKED_DOWN,
            },
        })
        data = recordsOrPending(
            args.loading
                ? {}
                : {
                      [RESOURCE]: [
                          electionRecord(undefined, {permission_label: args.label ?? null}),
                      ],
                      sequent_backend_election_event: [event],
                  }
        )
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        // The dashboard builds the voting portal addresses with sequent-core.
        await Promise.all([graphql.ready, initCore()])
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tabLabel = {
    dashboard: () => i18n.t("electionScreen.tabs.dashboard"),
    data: () => i18n.t("electionScreen.tabs.data"),
    voters: () => i18n.t("electionScreen.tabs.voters"),
    publish: () => i18n.t("electionScreen.tabs.publish"),
    approvals: () => i18n.t("electionScreen.tabs.approvals"),
    tallySheets: () => i18n.t("electionScreen.tabs.tallySheets"),
}

const dashboardDefects = {
    expectedFailure: {
        reason: "The election dashboard's loading spinner is a progressbar without an accessible name.",
        a11y: ["aria-progressbar-name"],
    },
}

/** The dashboard shows its spinner while the election's stats load. */
async function dashboardLoads(canvasElement: HTMLElement) {
    await waitFor(
        () => expect(graphql.calls.map((call) => call.name)).toContain("GetElectionStats"),
        {timeout: LAZY_LOAD_MS}
    )
    await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
}

const noPermission = () => i18n.t("electionScreen.common.noPermission")

const tabNames = async (canvasElement: HTMLElement) => {
    await within(canvasElement).findAllByRole("tab")
    return within(canvasElement)
        .getAllByRole("tab")
        .map((tab) => tab.textContent)
}

/** Selects the tab and waits for its widget to replace the fallback. */
async function openTab(canvasElement: HTMLElement, name: string) {
    const tab = await within(canvasElement).findByRole("tab", {name})
    await userEvent.click(tab)
    await waitFor(() => expect(tab).toHaveAttribute("aria-selected", "true"))
    await waitFor(
        () => expect(within(canvasElement).queryByText(/^Loading .*\.\.\.$/)).toBeNull(),
        {timeout: LAZY_LOAD_MS}
    )
}

interface TabContent {
    /** Text the tab's widget shows while its reads are pending. */
    text?: string | RegExp
    /** Data provider reads ("method resource") the widget starts. */
    reads?: string[]
    /** GraphQL operations the widget starts. */
    operations?: string[]
}

const tabPlay =
    (label: () => string, {text, reads = [], operations = []}: TabContent): Story["play"] =>
    async ({canvasElement}) => {
        await openTab(canvasElement, label())
        if (text) {
            await expect(await within(canvasElement).findByText(text)).toBeVisible()
        }
        await waitFor(() => {
            expect(readsOf(data)).toEqual(expect.arrayContaining(reads))
            expect(graphql.calls.map((call) => call.name)).toEqual(
                expect.arrayContaining(operations)
            )
        })
    }

export const Populated: Story = {
    parameters: dashboardDefects,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council")).toBeVisible()
        expect(await tabNames(canvasElement)).toEqual(Object.values(tabLabel).map((t) => t()))
        await expect(canvas.getByRole("tab", {name: tabLabel.dashboard()})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await dashboardLoads(canvasElement)
    },
}

export const Loading: Story = {
    args: {loading: true},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        // Until the election arrives the tabs show the no-permission message.
        await expect(await within(canvasElement).findByText(noPermission())).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(readsOf(data)).toEqual([`getOne ${RESOURCE}`])
    },
}

export const OutsideThePermissionLabel: Story = {
    args: {label: "north", userLabels: ["south"]},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The tabs read the election's event once the election has loaded.
        await waitFor(() =>
            expect(readsOf(data)).toContain("getOne sequent_backend_election_event")
        )
        await waitFor(() => {
            expect(canvas.queryAllByRole("tab")).toEqual([])
            expect(canvas.getByText(noPermission())).toBeVisible()
        })
        expect(canvas.queryByText("Council")).toBeNull()
        // No tab mounts, so nothing about the election is queried.
        expect(graphql.calls.map((call) => call.name)).not.toContain("GetElectionStats")
    },
}

export const InsideThePermissionLabel: Story = {
    args: {label: "north", userLabels: ["north"]},
    parameters: dashboardDefects,
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Council")).toBeVisible()
        expect(await tabNames(canvasElement)).toContain(tabLabel.dashboard())
        await dashboardLoads(canvasElement)
    },
}

export const LockedDownEvent: Story = {
    args: {eventLockedDown: true},
    parameters: dashboardDefects,
    play: async ({canvasElement}) => {
        // The event's lockdown hides the approvals once the event has loaded.
        await waitFor(() =>
            expect(readsOf(data)).toContain("getOne sequent_backend_election_event")
        )
        await waitFor(async () =>
            expect(await tabNames(canvasElement)).toEqual([
                tabLabel.dashboard(),
                tabLabel.data(),
                tabLabel.voters(),
                tabLabel.publish(),
                tabLabel.tallySheets(),
            ])
        )
        await dashboardLoads(canvasElement)
    },
}

export const OnlyTallySheets: Story = {
    args: {roles: [IPermissions.TALLY_SHEET_VIEW]},
    parameters: {widgets: ["TallySheetsTab"]},
    play: async ({canvasElement}) => {
        expect(await tabNames(canvasElement)).toEqual([tabLabel.tallySheets()])
    },
}

export const WithoutPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Council")).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const Dashboard: Story = {
    parameters: {...dashboardDefects, widgets: ["DashboardTab"]},
    play: async ({canvasElement}) => {
        await openTab(canvasElement, tabLabel.dashboard())
        await dashboardLoads(canvasElement)
    },
}
export const Data: Story = {
    parameters: {widgets: ["DataTab"]},
    play: tabPlay(tabLabel.data, {
        reads: ["getList sequent_backend_contest"],
    }),
}

export const Voters: Story = {
    parameters: {widgets: ["VotersTab"]},
    play: tabPlay(tabLabel.voters, {
        text: "Ext. voters sync",
        reads: ["getList user"],
        operations: ["GetUserProfileConfiguration"],
    }),
}

export const Publish: Story = {
    parameters: {widgets: ["PublishTab"]},
    play: tabPlay(tabLabel.publish, {
        text: "Publish History",
        reads: ["getList sequent_backend_ballot_publication"],
    }),
}

export const Approvals: Story = {
    parameters: {widgets: ["ApprovalsTab"]},
    play: tabPlay(tabLabel.approvals, {
        operations: ["getUserProfileAttributes"],
    }),
}

export const TallySheets: Story = {
    parameters: {widgets: ["TallySheetsTab"]},
    play: tabPlay(tabLabel.tallySheets, {
        text: "Digitalized ballot boxes by channel",
        reads: ["getList sequent_backend_tally_sheet"],
    }),
}
