// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {tallyExecution} from "./__stories__/TallyFixture"
import {TallyLogs} from "./TallyLogs"
import {
    EStoryWorkflow,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the tally has an execution whose status carries the logs. */
    executed: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({executed}: Scenario) {
    const {workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={boundary}>
            <TallyLogs tallySessionExecution={executed ? tallyExecution(workflow) : undefined} />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyLogs",
    component: TallyLogs,
    args: {executed: true},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The text of each log row, below the table's header row. */
const logRows = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("row")
        .slice(1)
        .map((row) => row.textContent)

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const {workflow} = readStoryGlobals(globals)
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("keysGeneration.ceremonyStep.logsHeader.title"))
        ).toBeVisible()
        expect(logRows(canvasElement)).toEqual([
            "1/15/2026, 12:00:00 PMTally session created",
            ...(workflow === EStoryWorkflow.RESULTS
                ? ["1/15/2026, 12:30:00 PMTally completed"]
                : []),
        ])
    },
}

export const Empty: Story = {
    args: {executed: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("keysGeneration.ceremonyStep.emptyLogs"))
        ).toBeVisible()
        expect(canvas.queryByRole("table")).toBeNull()
    },
}

export const CollapseTheLogs: Story = {
    globals: {workflow: EStoryWorkflow.RESULTS},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const summary = canvas.getByRole("button", {
            name: i18n.t("keysGeneration.ceremonyStep.logsHeader.title"),
        })
        await expect(summary).toHaveAttribute("aria-expanded", "true")
        await userEvent.click(summary)
        await waitFor(() => expect(summary).toHaveAttribute("aria-expanded", "false"))
        await waitFor(() => expect(canvas.getByText("Tally completed")).not.toBeVisible())
    },
}
