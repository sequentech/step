// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {EElectionEventWeightedVotingPolicy, ICountingAlgorithm, i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventPresentation, eventRecord} from "@/__stories__/fixtures"
import {TallyStoryContext, tallyData, tallyExecution} from "./__stories__/TallyFixture"
import {TallyResultsSectionArea} from "./TallyResultsSectionArea"
import {EStoryWorkflow, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    area: "north" | "south"
    countingAlgorithm: ICountingAlgorithm
    /** The event weights each area's votes. */
    weightedAreas: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({area, countingAlgorithm, weightedAreas}: Scenario) {
    const {workflow} = useStoryGlobals()
    const event = eventRecord(workflow, {
        presentation: {
            ...eventPresentation,
            weighted_voting_policy: weightedAreas
                ? EElectionEventWeightedVotingPolicy.AREAS_WEIGHTED_VOTING
                : EElectionEventWeightedVotingPolicy.DISABLED_WEIGHTED_VOTING,
        },
    })
    const preferential = countingAlgorithm === ICountingAlgorithm.INSTANT_RUNOFF
    return (
        <AdminStoryProvider boundary={boundary}>
            <RecordContextProvider value={event}>
                <TallyStoryContext
                    data={workflow === EStoryWorkflow.RESULTS ? tallyData({preferential}) : null}
                >
                    <TallyResultsSectionArea
                        areaId={area === "north" ? STORY_IDS.area : STORY_IDS.secondArea}
                        contestId={STORY_IDS.contest}
                        electionId={STORY_IDS.election}
                        electionEventId={EVENT_ID}
                        tenantId={TENANT_ID}
                        resultsEventId={tallyExecution(workflow).results_event_id ?? null}
                        counting_algorithm={countingAlgorithm}
                    />
                </TallyStoryContext>
            </RecordContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyResultsSectionArea",
    component: TallyResultsSectionArea,
    args: {
        area: "north",
        countingAlgorithm: ICountingAlgorithm.PLURALITY_AT_LARGE,
        weightedAreas: false,
    },
    argTypes: {
        area: {control: "inline-radio", options: ["north", "south"]},
        countingAlgorithm: {control: "select", options: Object.values(ICountingAlgorithm)},
    },
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

const candidateCells = (canvasElement: HTMLElement, alias: RegExp) =>
    cells(within(canvasElement).getByRole("row", {name: alias}), "gridcell")

const summaryRow = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).queryByRole("row", {name: new RegExp(i18n.t(label))})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        // The north district counted 56 of its 70 voters' valid votes: Alice 33, Bob 23.
        expect(
            cells(summaryRow(canvasElement, "tally.table.total_votes_counted") as HTMLElement)
        ).toEqual(["60", "85.71%"])
        expect(candidateCells(canvasElement, /^Alice/)).toEqual(["Alice", "33", "58.93%", "1"])
        expect(candidateCells(canvasElement, /^Bob/)).toEqual(["Bob", "23", "41.07%", "2"])
        await expect(
            within(canvasElement).getAllByText("Council - Members - North district", {
                exact: false,
            })[0]
        ).toBeVisible()
        // Weights are shown only when the event weights its areas.
        expect(summaryRow(canvasElement, "tally.table.weight")).toBeNull()
        expect(boundary.calls).toEqual([])
    },
}

export const OtherArea: Story = {
    args: {area: "south"},
    play: async ({canvasElement}) => {
        expect(candidateCells(canvasElement, /^Alice/)).toEqual(["Alice", "17", "60.71%", "1"])
        expect(candidateCells(canvasElement, /^Bob/)).toEqual(["Bob", "11", "39.29%", "2"])
    },
}

export const WeightedAreas: Story = {
    args: {weightedAreas: true},
    play: async ({canvasElement}) => {
        const weight = summaryRow(canvasElement, "tally.table.weight")
        await expect(weight).toBeVisible()
        expect(cells(weight as HTMLElement)).toContain("2")
    },
}

export const TallyInProgress: Story = {
    globals: {workflow: EStoryWorkflow.TALLY},
    play: async ({canvasElement}) => {
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("table")).toBeNull()
    },
}
