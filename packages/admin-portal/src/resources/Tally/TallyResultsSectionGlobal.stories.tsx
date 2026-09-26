// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ICountingAlgorithm, i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, storyId} from "@/__stories__/fixtures"
import {TallyStoryContext, tallyData, tallyExecution} from "./__stories__/TallyFixture"
import {TallyResultsSectionGlobal} from "./TallyResultsSectionGlobal"
import {EStoryWorkflow, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** The contest whose results are shown. */
    contest: "council" | "deputy"
    countingAlgorithm: ICountingAlgorithm
    acclaimed: boolean
    /** Whether the loaded results belong to another results event than the tally's. */
    stale: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({contest, countingAlgorithm, acclaimed, stale}: Scenario) {
    const {workflow} = useStoryGlobals()
    const preferential = countingAlgorithm === ICountingAlgorithm.INSTANT_RUNOFF
    // Results exist once the tally has completed; until then nothing is loaded.
    const results =
        workflow === EStoryWorkflow.RESULTS
            ? tallyData({
                  preferential,
                  acclaimed,
                  resultsEventId: stale ? storyId(9, 0) : undefined,
              })
            : null
    return (
        <AdminStoryProvider boundary={boundary}>
            <TallyStoryContext data={results}>
                <TallyResultsSectionGlobal
                    contestId={contest === "council" ? STORY_IDS.contest : STORY_IDS.secondContest}
                    electionId={
                        contest === "council" ? STORY_IDS.election : STORY_IDS.secondElection
                    }
                    electionEventId={EVENT_ID}
                    tenantId={TENANT_ID}
                    resultsEventId={tallyExecution(workflow).results_event_id ?? null}
                    counting_algorithm={countingAlgorithm}
                />
            </TallyStoryContext>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyResultsSectionGlobal",
    component: TallyResultsSectionGlobal,
    args: {
        contest: "council",
        countingAlgorithm: ICountingAlgorithm.PLURALITY_AT_LARGE,
        acclaimed: false,
        stale: false,
    },
    argTypes: {
        contest: {control: "inline-radio", options: ["council", "deputy"]},
        countingAlgorithm: {control: "select", options: Object.values(ICountingAlgorithm)},
    },
    // The section shows the results of a completed tally.
    globals: {workflow: EStoryWorkflow.RESULTS},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const cells = (row: HTMLElement, role: "cell" | "gridcell" = "cell") =>
    within(row)
        .getAllByRole(role)
        .map((cell) => cell.textContent)

// Candidate results are a data grid; candidates are shown by their alias.
const candidateCells = (canvasElement: HTMLElement, alias: RegExp) =>
    cells(within(canvasElement).getByRole("row", {name: alias}), "gridcell")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // 90 of the 120 eligible voters voted; Alice has 50 and Bob 34 of the 84 valid votes.
        expect(
            cells(
                canvas.getByRole("row", {
                    name: new RegExp(i18n.t("tally.table.total_votes_counted")),
                })
            )
        ).toEqual(["90", "75.00%"])
        expect(candidateCells(canvasElement, /^Alice/)).toEqual(["Alice", "50", "59.52%", "1"])
        expect(candidateCells(canvasElement, /^Bob/)).toEqual(["Bob", "34", "40.48%", "2"])
        // The section reads the loaded results only; it sends no request of its own.
        expect(boundary.calls).toEqual([])
    },
}

export const OtherContest: Story = {
    args: {contest: "deputy"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(candidateCells(canvasElement, /Carol Example/)).toEqual([
            "Carol Example",
            "21",
            "55.26%",
            "1",
        ])
        expect(canvas.queryByRole("row", {name: /^Alice/})).toBeNull()
    },
}

export const InstantRunoffRounds: Story = {
    args: {countingAlgorithm: ICountingAlgorithm.INSTANT_RUNOFF},
    parameters: {
        expectedFailure: {
            reason: "The shared preferential results table's winner chip has white text on the success green.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("columnheader", {
                name: new RegExp(i18n.t("tally.table.preferential.round")),
            })
        ).toBeVisible()
        await expect(canvas.getByText("50 (59.52%)")).toBeVisible()
    },
}

export const Acclaimed: Story = {
    args: {acclaimed: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("tally.table.acclamation_note"))).toBeVisible()
        expect(
            canvas.queryByRole("row", {name: new RegExp(i18n.t("tally.table.total_votes_counted"))})
        ).toBeNull()
    },
}

export const StaleResultsKeepLoading: Story = {
    args: {stale: true},
    play: async ({canvasElement}) => {
        // Results of another results event are never shown for this tally.
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("row", {name: /^Alice/})).toBeNull()
    },
}

export const TallyInProgress: Story = {
    globals: {workflow: EStoryWorkflow.TALLY},
    play: async ({canvasElement}) => {
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("table")).toBeNull()
    },
}
