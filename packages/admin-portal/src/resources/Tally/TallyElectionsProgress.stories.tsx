// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Election} from "@/gql/graphql"
import {
    COUNCIL_ELECTION,
    DEPUTY_ELECTION,
    tallyExecution,
    tallySession,
} from "./__stories__/TallyFixture"
import {TallyElectionsProgress} from "./TallyElectionsProgress"
import {
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the tally exists; before the tally step there is none. */
    tally: boolean
    /** Whether the tally's executions have loaded; they report each election's progress. */
    executed: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({tally, executed}: Scenario) {
    const {workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={boundary}>
            <TallyElectionsProgress
                tally={tally ? tallySession(workflow) : undefined}
                tallySessionExecutions={executed ? [tallyExecution(workflow)] : undefined}
                allElections={[COUNCIL_ELECTION, DEPUTY_ELECTION] as Sequent_Backend_Election[]}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyElectionsProgress",
    component: TallyElectionsProgress,
    args: {tally: true, executed: true},
    parameters: {
        expectedFailure: {
            reason: "The elections' progress bars have no accessible name and their status chips have white text below 4.5 contrast.",
            a11y: ["aria-progressbar-name", "color-contrast"],
        },
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The status and progress cells of an election's row. */
async function progress(canvasElement: HTMLElement, election: string) {
    const row = await within(canvasElement).findByRole("row", {name: new RegExp(election)})
    return within(row)
        .getAllByRole("gridcell")
        .slice(1)
        .map((cell) => cell.textContent)
}

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        // While the ceremony runs the council election mixes and the deputy one waits.
        const completed = readStoryGlobals(globals).workflow === EStoryWorkflow.RESULTS
        expect(await progress(canvasElement, "Council")).toEqual(
            completed ? ["SUCCESS", "100.00%"] : ["MIXING", "40.00%"]
        )
        expect(await progress(canvasElement, "Deputy")).toEqual(
            completed ? ["SUCCESS", "100.00%"] : ["WAITING", "0.00%"]
        )
    },
}

export const TallyCompleted: Story = {
    globals: {workflow: EStoryWorkflow.RESULTS},
    play: async ({canvasElement}) => {
        const bars = await within(canvasElement).findAllByRole("progressbar")
        expect(bars.map((bar) => bar.getAttribute("aria-valuenow"))).toEqual(["100", "100"])
    },
}

export const ExecutionsLoading: Story = {
    args: {executed: false},
    play: async ({canvasElement}) => {
        expect(await progress(canvasElement, "Council")).toEqual(["WAITING", "0.00%"])
        expect(await progress(canvasElement, "Deputy")).toEqual(["WAITING", "0.00%"])
    },
}

export const Empty: Story = {
    args: {tally: false, executed: false},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No rows")).toBeVisible()
    },
}
