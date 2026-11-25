// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETallyFailureReason, type ITallyCeremonyStatus} from "@/types/ceremonies"
import {tallyExecution} from "./__stories__/TallyFixture"
import {TallyFailureAlert} from "./TallyFailureAlert"
import {useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const DETAIL = "area North: 2 sealed ballot(s) don't open"

interface Scenario {
    /** The reason the execution's status records; none for a tally that didn't fail. */
    reason?: ETallyFailureReason
}

let boundary: ReturnType<typeof graphqlBoundary>

function Fixture({reason}: Scenario) {
    const {workflow} = useStoryGlobals()
    const execution = tallyExecution(workflow)
    const status: ITallyCeremonyStatus = {
        ...(execution.status as ITallyCeremonyStatus),
        failure: reason ? {reason, detail: DETAIL, failed_at: "2026-01-15T12:00:00Z"} : null,
    }
    return (
        <AdminStoryProvider boundary={boundary}>
            <TallyFailureAlert tallySessionExecution={{...execution, status}} />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyFailureAlert",
    component: TallyFailureAlert,
    args: {reason: ETallyFailureReason.SEALED_BALLOTS_REFUSED},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const SealedBallotsRefused: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("tally.failure.title"))).toBeVisible()
        await expect(
            canvas.getByText(i18n.t("tally.failure.reason.SEALED_BALLOTS_REFUSED"))
        ).toBeVisible()
        await expect(canvas.getByText(`${i18n.t("tally.failure.detail")}: ${DETAIL}`)).toBeVisible()
    },
}

export const TrusteeHalted: Story = {
    args: {reason: ETallyFailureReason.TRUSTEE_HALTED},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("tally.failure.reason.TRUSTEE_HALTED"))
        ).toBeVisible()
    },
}

export const NotFailed: Story = {
    args: {reason: undefined},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
    },
}
