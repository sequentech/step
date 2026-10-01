// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {Route, Routes} from "react-router"
import {EElectionEventLockedDown, i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {eventPresentation, eventRecord} from "@/__stories__/fixtures"
import type {ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventBaseTabs} from "./ElectionEventBaseTabs"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

interface Scenario {
    /** What reading the election event does. */
    reads: ReadState
    /** Whether the event is locked down. */
    lockedDown: boolean
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <ResourceContextProvider value={RESOURCE}>
                <Routes>
                    <Route path={`/${RESOURCE}/:id/*`} element={<ElectionEventBaseTabs />} />
                </Routes>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

const spinnerDefect = {
    expectedFailure: {
        reason: "The loading spinner is a progressbar without an accessible name.",
        a11y: ["aria-progressbar-name"],
    },
}

// The tabs' contents have their own sections; their reads stay loading here.
const meta = {
    title: "Admin/Election event/ElectionEventBaseTabs",
    component: ElectionEventBaseTabs,
    args: {reads: "records", lockedDown: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {initialEntries: [`/${RESOURCE}/${EVENT_ID}/tally`]},
        ...spinnerDefect,
    },
    beforeEach: async ({args}) => {
        const presentation = args.lockedDown
            ? {...eventPresentation, locked_down: EElectionEventLockedDown.LOCKED_DOWN}
            : eventPresentation
        data = recordsOrPending(
            {[RESOURCE]: [eventRecord(undefined, {presentation})]},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        // The dashboard builds the voting portal addresses with sequent-core.
        await Promise.all([graphql.ready, initCore()])
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tabLabels = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("tab")
        .map((tab) => tab.textContent)

/**
 * The first tab, the event dashboard: for an administrator with monitoring,
 * it first asks whether the event's monitoring dashboards are configured, and
 * shows its named loader until the answer.
 *
 * The dashboard is a lazy chunk, and the file's first story fetches it cold, alongside
 * the portal's Roboto faces: that takes longer than Testing Library's one second.
 */
async function dashboardLoading(canvasElement: HTMLElement) {
    await expect(
        await within(canvasElement).findByRole(
            "progressbar",
            {name: i18n.t("monitoring.loading")},
            {timeout: 5000}
        )
    ).toBeVisible()
    await waitFor(() =>
        expect(graphql.calls).toContainEqual(
            expect.objectContaining({
                name: "MonitoringListDashboards",
                variables: expect.objectContaining({electionEventId: EVENT_ID}),
            })
        )
    )
}

const labels = (...keys: string[]) => keys.map((key) => i18n.t(`electionEventScreen.tabs.${key}`))

/** The Signatures tab, labelled by the signing feature's own key. */
const signatures = () => i18n.t("signing.tab.title")

export const Populated: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    // The monitoring loader has a name.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council")).toBeVisible()
        expect(tabLabels(canvasElement)).toEqual([
            ...labels("dashboard", "data", "localization", "voters", "areas", "keys"),
            signatures(),
            ...labels(
                "tally",
                "tallySheetImports",
                "publish",
                "tasks",
                "logs",
                "events",
                "reports",
                "approvals"
            ),
        ])
        expect(paramsOf(data, "getOne", RESOURCE)).toMatchObject({id: EVENT_ID})
        await dashboardLoading(canvasElement)
        // Once the event has loaded, the address returns to the event itself.
        await waitFor(() =>
            expect(canvas.getByLabelText("Current location").textContent).toBe(
                `/${RESOURCE}/${EVENT_ID}`
            )
        )
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("progressbar")).toBeVisible()
        expect(canvas.queryByRole("tab")).toBeNull()
        expect(paramsOf(data, "getOne", RESOURCE)).toMatchObject({id: EVENT_ID})
        expect(canvas.getByLabelText("Current location")).toHaveTextContent(
            `/${RESOURCE}/${EVENT_ID}/tally`
        )
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // React-admin reports the missing event and returns to the event list.
        await expect(await within(document.body).findByText("Element does not exist")).toBeVisible()
        await waitFor(() =>
            expect(canvas.getByLabelText("Current location").textContent).toBe(`/${RESOURCE}`)
        )
        expect(canvas.queryByRole("tab")).toBeNull()
        expect(paramsOf(data, "getOne", RESOURCE)).toMatchObject({id: EVENT_ID})
    },
}

export const LockedDownEvent: Story = {
    args: {lockedDown: true},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Council")).toBeVisible()
        // The Signatures tab stays, read-only, after lockdown.
        expect(tabLabels(canvasElement)).toEqual([
            ...labels("dashboard", "voters"),
            signatures(),
            ...labels("logs", "reports"),
        ])
        await dashboardLoading(canvasElement)
    },
}

export const TrusteeTabs: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("Council")).toBeVisible()
        expect(tabLabels(canvasElement)).toEqual(labels("keys", "tally"))
    },
}
