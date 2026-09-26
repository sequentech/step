// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EDeclineToVotePolicy, i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, electionPresentation, storyId} from "@/__stories__/fixtures"
import type {GetTallyDataQuery} from "@/gql/graphql"
import {TallyStoryContext, TALLY_IDS, tallyData} from "./__stories__/TallyFixture"
import {TallyElectionsResults} from "./TallyElectionsResults"

interface Scenario {
    /** The loaded results: the tally's, another results event's, or none yet. */
    results: "loaded" | "stale" | "none" | "no-elections"
    /** The council election asks voters whether they decline to vote. */
    declineToVote: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>

function loadedResults({results, declineToVote}: Scenario): GetTallyDataQuery | null {
    if (results === "none") return null
    const data = tallyData()
    if (results === "stale") {
        const otherEvent = storyId(9, 0)
        return {
            ...data,
            sequent_backend_results_event: data.sequent_backend_results_event.map((event) => ({
                ...event,
                id: otherEvent,
            })),
            sequent_backend_results_election: data.sequent_backend_results_election.map(
                (result) => ({...result, results_event_id: otherEvent})
            ),
        }
    }
    if (results === "no-elections") return {...data, sequent_backend_election: []}
    if (!declineToVote) return data
    return {
        ...data,
        sequent_backend_election: data.sequent_backend_election.map((election) =>
            election.id === STORY_IDS.election
                ? {
                      ...election,
                      presentation: JSON.stringify({
                          ...electionPresentation("Council election"),
                          decline_to_vote_policy: EDeclineToVotePolicy.ENABLED,
                      }),
                  }
                : election
        ),
        sequent_backend_results_contest: data.sequent_backend_results_contest.map((contest) =>
            contest.election_id === STORY_IDS.election
                ? {
                      ...contest,
                      annotations: JSON.stringify({extended_metrics: {total_declined_to_vote: 4}}),
                  }
                : contest
        ),
    }
}

const meta = {
    title: "Admin/Tally/TallyElectionsResults",
    component: TallyElectionsResults,
    args: {results: "loaded", declineToVote: false},
    argTypes: {
        results: {control: "inline-radio", options: ["loaded", "stale", "none", "no-elections"]},
    },
    parameters: {
        widgets: ["GeneralInformationCharts"],
        expectedFailure: {
            reason: "The participation chart's collapse toggle is an unnamed icon button.",
            a11y: ["button-name"],
        },
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <TallyStoryContext data={loadedResults(args)}>
                <TallyElectionsResults
                    tenantId={TENANT_ID}
                    electionEventId={EVENT_ID}
                    resultsEventId={TALLY_IDS.resultsEvent}
                    electionIds={[STORY_IDS.election, STORY_IDS.secondElection]}
                    isMultiContest={args.declineToVote}
                />
            </TallyStoryContext>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const gridCells = (row: HTMLElement) =>
    within(row)
        .getAllByRole("gridcell")
        .map((cell) => cell.textContent)

const chartTitle = (canvasElement: HTMLElement) =>
    canvasElement.querySelector(".seq-admin-tally-results__general-information-chart")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // 90 of the council's 120 voters and 40 of the deputies' 80 voted.
        expect(gridCells(await canvas.findByRole("row", {name: /^Council/}))).toEqual([
            "Council",
            "120",
            "90",
            "75.00%",
        ])
        expect(gridCells(canvas.getByRole("row", {name: /^Deputy/}))).toEqual([
            "Deputy",
            "80",
            "40",
            "50.00%",
        ])
        // The participation chart shows the first election until another is chosen.
        await expect(
            within(chartTitle(canvasElement) as HTMLElement).getByText("Council")
        ).toBeVisible()
        expect(
            canvas.queryByRole("columnheader", {name: i18n.t("tally.table.total_declined_to_vote")})
        ).toBeNull()
        expect(boundary.calls).toEqual([])
    },
}

export const ChoosingAnElectionShowsItsParticipation: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const deputy = await canvas.findByRole("row", {name: /^Deputy/})
        await userEvent.click(within(deputy).getAllByRole("gridcell")[1])
        const chart = within(chartTitle(canvasElement) as HTMLElement)
        await waitFor(() => expect(chart.getByText("Deputy")).toBeVisible())
        expect(chart.queryByText("Council")).toBeNull()
        await expect(deputy).toHaveClass("selected")
    },
}

export const DeclinedToVoteColumn: Story = {
    args: {declineToVote: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("columnheader", {
                name: i18n.t("tally.table.total_declined_to_vote"),
            })
        ).toBeVisible()
        expect(gridCells(canvas.getByRole("row", {name: /^Council/}))).toEqual([
            "Council",
            "120",
            "90",
            "4",
            "75.00%",
        ])
        // The deputy election has no such policy.
        expect(gridCells(canvas.getByRole("row", {name: /^Deputy/}))[3]).toBe("-")
    },
}

export const WaitingForResults: Story = {
    args: {results: "none"},
    parameters: {widgets: ["LoadingResults"], expectedFailure: null},
    play: async ({canvasElement}) => {
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("grid")).toBeNull()
    },
}

export const StaleResultsKeepLoading: Story = {
    args: {results: "stale"},
    parameters: {widgets: ["LoadingResults"], expectedFailure: null},
    play: async ({canvasElement}) => {
        // Results of another results event are never shown for this tally.
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("row", {name: /^Council/})).toBeNull()
    },
}

export const NoElectionResults: Story = {
    args: {results: "no-elections"},
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(i18n.t("common.label.noResult"))
        ).toBeVisible()
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).toBeNull()
        expect(within(canvasElement).queryByRole("grid")).toBeNull()
        expect(chartTitle(canvasElement)).toBeNull()
    },
}
