// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn} from "storybook/test"
import {EGracePeriodPolicy} from "@sequentech/ui-core"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ManagedSelectInput} from "./ManagedSelectInput"
import {
    GracePeriodForm,
    choosePolicy,
    policyName,
    policySelect,
    savedPresentation,
    setUpGracePeriodForm,
    type GracePeriodScenario,
} from "./__stories__/GracePeriodFixture"

const meta = {
    title: "Admin/Managed inputs/ManagedSelectInput",
    component: ManagedSelectInput,
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

export const DefaultForANewElection: Story = {
    play: async ({canvasElement, args}) => {
        await expect(policySelect(canvasElement)).toHaveTextContent(
            policyName(EGracePeriodPolicy.NO_GRACE_PERIOD)
        )
        // Returning to the default after trying another policy saves the default value.
        await choosePolicy(canvasElement, EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT)
        await choosePolicy(canvasElement, EGracePeriodPolicy.NO_GRACE_PERIOD)
        expect(await savedPresentation(canvasElement, args.onSubmit)).toMatchObject({
            grace_period_policy: EGracePeriodPolicy.NO_GRACE_PERIOD,
        })
    },
}

export const SavedPolicy: Story = {
    args: {policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT, seconds: 60},
    play: async ({canvasElement}) => {
        await expect(policySelect(canvasElement)).toHaveTextContent(
            policyName(EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT)
        )
    },
}

export const ChooseAnotherPolicy: Story = {
    args: {policy: EGracePeriodPolicy.GRACE_PERIOD_WITHOUT_ALERT, seconds: 60},
    play: async ({canvasElement, args}) => {
        await choosePolicy(canvasElement, EGracePeriodPolicy.NO_GRACE_PERIOD)
        await expect(policySelect(canvasElement)).toHaveTextContent(
            policyName(EGracePeriodPolicy.NO_GRACE_PERIOD)
        )
        expect(await savedPresentation(canvasElement, args.onSubmit)).toMatchObject({
            grace_period_policy: EGracePeriodPolicy.NO_GRACE_PERIOD,
        })
    },
}
