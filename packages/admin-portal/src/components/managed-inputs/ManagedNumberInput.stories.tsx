// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent} from "storybook/test"
import {EGracePeriodPolicy} from "@sequentech/ui-core"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ManagedNumberInput} from "./ManagedNumberInput"
import {
    GracePeriodForm,
    choosePolicy,
    savedPresentation,
    secondsInput,
    setUpGracePeriodForm,
    type GracePeriodScenario,
} from "./__stories__/GracePeriodFixture"

const meta = {
    title: "Admin/Managed inputs/ManagedNumberInput",
    component: ManagedNumberInput,
    args: {onSubmit: fn()},
    argTypes: {
        policy: {control: "select", options: Object.values(EGracePeriodPolicy)},
        seconds: {control: "number"},
    },
    beforeEach: setUpGracePeriodForm,
    render: (args) => <GracePeriodForm {...args} />,
} satisfies WidgetMeta<GracePeriodScenario>
export default meta
type Story = StoryObj<GracePeriodScenario>

export const DisabledByThePolicy: Story = {
    args: {policy: EGracePeriodPolicy.NO_GRACE_PERIOD, seconds: 0},
    play: async ({canvasElement}) => {
        await expect(secondsInput(canvasElement)).toBeDisabled()
        await expect(secondsInput(canvasElement)).toHaveValue(0)
    },
}

export const EnabledByThePolicy: Story = {
    args: {policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT, seconds: 120},
    play: async ({canvasElement, args}) => {
        const seconds = secondsInput(canvasElement)
        await expect(seconds).toBeEnabled()
        await expect(seconds).toHaveValue(120)
        await userEvent.clear(seconds)
        await userEvent.type(seconds, "300")
        expect(await savedPresentation(canvasElement, args.onSubmit)).toEqual({
            grace_period_policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT,
            grace_period_secs: 300,
        })
    },
}

export const ChoosingAGracePeriodEnablesIt: Story = {
    play: async ({canvasElement, args}) => {
        // A new election starts without a grace period and with the default of 0 seconds.
        await expect(secondsInput(canvasElement)).toBeDisabled()
        await expect(secondsInput(canvasElement)).toHaveValue(0)
        await choosePolicy(canvasElement, EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT)
        const seconds = secondsInput(canvasElement)
        await expect(seconds).toBeEnabled()
        await userEvent.clear(seconds)
        await userEvent.type(seconds, "45")
        expect(await savedPresentation(canvasElement, args.onSubmit)).toEqual({
            grace_period_policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT,
            grace_period_secs: 45,
        })
    },
}
