// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {OUTCOME_KEY, ScheduledOutcome, outcomeNote} from "./ScheduledOutcome"
import {MyTimeZoneProvider} from "./timeZoneService"
import {MY_TIME_ZONE} from "./__fixtures__/configurations"
import {
    refusedDefaults,
    refusedEdited,
    refusedLooser,
    runsAuthorized,
    runsNoSignatures,
    runsUnsigned,
} from "./__fixtures__/explanations"

const EXPLANATIONS = {
    runsAuthorized,
    runsNoSignatures,
    runsUnsigned,
    refusedEdited,
    refusedLooser,
    refusedDefaults,
}

interface Scenario {
    explanation: keyof typeof EXPLANATIONS
    /** The row's zone, for times in the explanation. */
    zone: string
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Timezones/ScheduledOutcome",
    component: ScheduledOutcome,
    args: {explanation: "runsAuthorized", zone: "Asia/Manila"},
    argTypes: {explanation: {control: "select", options: Object.keys(EXPLANATIONS)}},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({explanation, zone}) => (
        <AdminStoryProvider boundary={boundary}>
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ScheduledOutcome explanation={EXPLANATIONS[explanation]()} zone={zone} />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The chip and note of the scenario's outcome, then its "Why?" with the deciding check. */
const showsOutcome = async (canvasElement: HTMLElement, explanation: keyof typeof EXPLANATIONS) => {
    const value = EXPLANATIONS[explanation]()
    const canvas = within(canvasElement)
    await expect(
        canvas.getByText(i18n.t(`scheduledOutcome.chip.${OUTCOME_KEY[value.outcome]}`))
    ).toBeVisible()
    await expect(canvas.getByText(outcomeNote(value, i18n.t))).toBeVisible()
    await userEvent.click(canvas.getByRole("button", {name: i18n.t("scheduledOutcome.why.button")}))
    const dialogElement = await within(document.body).findByRole("dialog")
    const panel = within(dialogElement)
    await expect(panel.getByTestId(`outcome-check-${value.deciding}`)).toHaveAttribute(
        "aria-current",
        "true"
    )
    // The popover fades in.
    await waitFor(() => expect(panel.getByText(i18n.t(value.next_step.message_key))).toBeVisible())
    await userEvent.keyboard("{Escape}")
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
}

export const WillRunAuthorized: Story = {
    parameters: {widgets: ["OutcomeChip", "OutcomeChecks"]},
    play: ({canvasElement}) => showsOutcome(canvasElement, "runsAuthorized"),
}
export const WillRunNoSignaturesNeeded: Story = {
    args: {explanation: "runsNoSignatures"},
    play: ({canvasElement}) => showsOutcome(canvasElement, "runsNoSignatures"),
}
export const WillRunWithoutSignatures: Story = {
    args: {explanation: "runsUnsigned"},
    play: ({canvasElement}) => showsOutcome(canvasElement, "runsUnsigned"),
}
export const RefusedEditedAfterSigning: Story = {
    args: {explanation: "refusedEdited"},
    play: ({canvasElement}) => showsOutcome(canvasElement, "refusedEdited"),
}
export const RefusedLooserSinceThePublication: Story = {
    args: {explanation: "refusedLooser"},
    play: ({canvasElement}) => showsOutcome(canvasElement, "refusedLooser"),
}
export const RefusedNothingPublished: Story = {
    args: {explanation: "refusedDefaults"},
    play: ({canvasElement}) => showsOutcome(canvasElement, "refusedDefaults"),
}
