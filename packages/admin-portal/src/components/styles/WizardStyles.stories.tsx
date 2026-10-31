// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CustomToolbar, WizardStyles} from "./WizardStyles"

interface Scenario {
    /** Whether the current wizard step is complete. */
    canContinue: boolean
    onBack: () => void
    onNext: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Styles/CustomToolbar",
    component: CustomToolbar,
    args: {canContinue: true, onBack: fn(), onNext: fn()},
    beforeEach: async () => {
        boundary = graphqlBoundary({}, {schema: true})
        await boundary.ready
    },
    // A wizard step's footer, as the keys ceremony check step renders it.
    render: ({canContinue, onBack, onNext}) => (
        <AdminStoryProvider boundary={boundary}>
            <CustomToolbar>
                <WizardStyles.BackButton color="info" onClick={onBack}>
                    Back
                </WizardStyles.BackButton>
                <WizardStyles.NextButton disabled={!canContinue} color="info" onClick={onNext}>
                    Next
                </WizardStyles.NextButton>
            </CustomToolbar>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WizardFooter: Story = {
    play: async ({canvasElement, args}) => {
        const toolbar = within(canvasElement).getByRole("toolbar")
        await userEvent.click(within(toolbar).getByRole("button", {name: "Next"}))
        await userEvent.click(within(toolbar).getByRole("button", {name: "Back"}))
        expect(args.onNext).toHaveBeenCalledTimes(1)
        expect(args.onBack).toHaveBeenCalledTimes(1)
        expect(toolbar.closest(".MuiPaper-elevation2")).not.toBeNull()
    },
}

export const StepIncomplete: Story = {
    args: {canContinue: false},
    play: async ({canvasElement, args}) => {
        const toolbar = within(canvasElement).getByRole("toolbar")
        await expect(within(toolbar).getByRole("button", {name: "Next"})).toBeDisabled()
        await userEvent.click(within(toolbar).getByRole("button", {name: "Back"}))
        expect(args.onBack).toHaveBeenCalledTimes(1)
        expect(args.onNext).not.toHaveBeenCalled()
    },
}
