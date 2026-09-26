// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider, ShowBase} from "react-admin"
import {
    EElectionEventLockedDown,
    EVoterCertificatePolicy,
    initCore,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {eventPresentation, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTabs} from "./ElectionEventTabs"
import {answerOrPending, readsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {jsonEditorDefects} from "./__stories__/IvrFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the event is locked down, which hides its editing tabs. */
    lockedDown: boolean
    /** Whether the event has no record yet. */
    loading: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

/** An event with telephone voting and voter certificates, which show the IVR and CA tabs. */
const tabsEvent = (lockedDown: boolean) => {
    const presentation: IElectionEventPresentation = {
        ...eventPresentation,
        voter_certificate_policy: EVoterCertificatePolicy.ENABLED,
        locked_down: lockedDown
            ? EElectionEventLockedDown.LOCKED_DOWN
            : EElectionEventLockedDown.NOT_LOCKED_DOWN,
    }
    return eventRecord(undefined, {
        presentation,
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: true},
    })
}

function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            {/* As in the event's route, whose cached record the tabs' edit forms reuse. */}
            <ResourceContextProvider value="sequent_backend_election_event">
                <ShowBase>
                    <ElectionEventTabs />
                </ShowBase>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

// Every tab's widget has its own section: its reads stay loading here, and
// each story asserts that the selected tab mounts that widget.
const meta = {
    title: "Admin/Election event/ElectionEventTabs",
    component: ElectionEventTabs,
    args: {lockedDown: false, loading: false},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        router: {
            path: "/sequent_backend_election_event/:id/*",
            initialEntries: [`/sequent_backend_election_event/${EVENT_ID}`],
        },
        widgets: ["DashboardTab"],
    },
    beforeEach: async ({args}) => {
        data = recordsOrPending(
            args.loading ? {} : {sequent_backend_election_event: [tabsEvent(args.lockedDown)]}
        )
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        // The dashboard builds the voting portal addresses with sequent-core.
        await Promise.all([graphql.ready, initCore()])
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const ADMIN_TABS = [
    "Dashboard",
    "Data",
    "IVR",
    "Localization",
    "Voters",
    "Areas",
    "Keys",
    "Certificates",
    "Tally",
    "Tally sheet imports",
    "Publish",
    "Tasks",
    "Logs",
    "Scheduled Events",
    "Reports",
    "Approvals",
]

/** The tabs load their widgets lazily, which a cold dev server serves slowly. */
const LAZY_LOAD_MS = 10_000

/** A widget's loading spinner is a progressbar without an accessible name. */
const spinnerDefects = (widget: string) => ({
    expectedFailure: {
        reason: `${widget}'s loading spinner is a progressbar without an accessible name.`,
        a11y: ["aria-progressbar-name"],
    },
})

/** The dashboard shows its spinner while the event's stats load. */
async function dashboardLoads(canvasElement: HTMLElement) {
    await waitFor(
        () => expect(graphql.calls.map((call) => call.name)).toContain("GetElectionEventStats"),
        {timeout: LAZY_LOAD_MS}
    )
    await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
}

const tabNames = async (canvasElement: HTMLElement) => {
    await within(canvasElement).findAllByRole("tab")
    return within(canvasElement)
        .getAllByRole("tab")
        .map((tab) => tab.textContent)
}

/** Selects the tab and waits for its lazily loaded widget to replace the fallback. */
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
    (name: string, {text, reads = [], operations = []}: TabContent): Story["play"] =>
    async ({canvasElement}) => {
        await openTab(canvasElement, name)
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
    parameters: {...spinnerDefects("The election event dashboard"), widgets: ["DashboardTab"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council")).toBeVisible()
        expect(await tabNames(canvasElement)).toEqual(ADMIN_TABS)
        await expect(canvas.getByRole("tab", {name: "Dashboard"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await dashboardLoads(canvasElement)
    },
}

export const LoadingRecord: Story = {
    args: {loading: true},
    parameters: {...spinnerDefects("ElectionEventTabs"), widgets: []},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(graphql.calls).toEqual([])
        expect(readsOf(data)).toEqual(["getOne sequent_backend_election_event"])
    },
}

export const LockedDownEvent: Story = {
    args: {lockedDown: true},
    parameters: spinnerDefects("The election event dashboard"),
    play: async ({canvasElement}) => {
        expect(await tabNames(canvasElement)).toEqual([
            "Dashboard",
            "IVR",
            "Voters",
            "Certificates",
            "Logs",
            "Reports",
        ])
        await dashboardLoads(canvasElement)
    },
}

export const TrusteeTabs: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE},
    parameters: {widgets: ["KeysTab"]},
    play: async ({canvasElement}) => {
        expect(await tabNames(canvasElement)).toEqual(["Keys", "Tally"])
    },
}

export const WithoutPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Council")).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(graphql.calls).toEqual([])
        expect(readsOf(data)).toEqual(["getOne sequent_backend_election_event"])
    },
}

export const TallySheetImportLink: Story = {
    parameters: {
        router: {
            path: "/sequent_backend_election_event/:id/*",
            initialEntries: [
                `/sequent_backend_election_event/${EVENT_ID}?tabId=tally-sheet-imports`,
            ],
        },
        widgets: ["TallySheetImportsTab"],
    },
    play: async ({canvasElement}) => {
        const tab = await within(canvasElement).findByRole("tab", {name: "Tally sheet imports"})
        await waitFor(() => expect(tab).toHaveAttribute("aria-selected", "true"))
    },
}

export const Data: Story = {
    parameters: {widgets: ["DataTab"]},
    play: tabPlay("Data", {reads: ["getList sequent_backend_election"]}),
}

export const Ivr: Story = {
    parameters: {...jsonEditorDefects, widgets: ["IvrTab"]},
    play: tabPlay("IVR", {text: /^Configure the IVR flow and its properties below/}),
}

export const Localization: Story = {
    parameters: {
        expectedFailure: {
            reason: "The translation rows' edit and delete icon buttons have no accessible name.",
            a11y: ["button-name"],
        },
        widgets: ["LocalizationTab"],
    },
    play: tabPlay("Localization", {}),
}

export const Voters: Story = {
    parameters: {widgets: ["VotersTab"]},
    play: tabPlay("Voters", {
        text: "Ext. voters sync",
        reads: ["getList user"],
        operations: ["GetUserProfileConfiguration"],
    }),
}

export const Areas: Story = {
    parameters: {widgets: ["AreasTab"]},
    play: tabPlay("Areas", {
        text: "Upsert Areas",
        reads: ["getList sequent_backend_area"],
    }),
}

export const Keys: Story = {
    parameters: {widgets: ["KeysTab"]},
    play: tabPlay("Keys", {
        reads: ["getList sequent_backend_keys_ceremony"],
        operations: ["TrusteeNames"],
    }),
}

export const Certificates: Story = {
    parameters: {widgets: ["CAsTab"]},
    play: tabPlay("Certificates", {
        text: /^Certificate Authorities \(CAs\) trusted for this election event/,
        reads: ["getList sequent_backend_certificate_authority"],
    }),
}

export const Tally: Story = {
    parameters: {widgets: ["TallyTab"]},
    play: tabPlay("Tally", {
        text: "Election Event Tally",
        reads: ["getList sequent_backend_tally_session_execution"],
    }),
}

export const TallySheetImports: Story = {
    parameters: {widgets: ["TallySheetImportsTab"]},
    play: tabPlay("Tally sheet imports", {
        text: "Import tally sheets",
        reads: ["getList sequent_backend_tally_sheet_import"],
    }),
}

export const Publish: Story = {
    parameters: {widgets: ["PublishTab"]},
    play: tabPlay("Publish", {
        text: "Publish History",
        reads: ["getList sequent_backend_ballot_publication"],
    }),
}

export const Tasks: Story = {
    parameters: {widgets: ["TasksTab"]},
    play: tabPlay("Tasks", {
        text: "Tasks Execution",
        reads: ["getList sequent_backend_tasks_execution"],
    }),
}

export const Logs: Story = {
    parameters: {widgets: ["LogsTab"]},
    play: tabPlay("Logs", {
        text: "General logs of the main and IAM databases",
        reads: ["getList electoral_log"],
    }),
}

export const ScheduledEvents: Story = {
    parameters: {widgets: ["EventsTab"]},
    play: tabPlay("Scheduled Events", {
        reads: ["getList sequent_backend_scheduled_event"],
    }),
}

export const Reports: Story = {
    parameters: {...spinnerDefects("ListReports"), widgets: ["ReportsTab"]},
    play: tabPlay("Reports", {reads: ["getList sequent_backend_template"]}),
}

export const Approvals: Story = {
    parameters: {widgets: ["ApprovalsTab"]},
    play: tabPlay("Approvals", {
        operations: ["getUserProfileAttributes"],
    }),
}
