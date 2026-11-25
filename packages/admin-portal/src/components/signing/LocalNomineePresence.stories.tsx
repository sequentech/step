// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {LocalOfficerApi} from "@/lib/signing/localInitializeOfficerApi"
import {LocalNomineePresence} from "./LocalInitializeEnrollment"

interface Scenario {
    onGold: () => Promise<void>
}

const DIGEST = "ab".repeat(32)
let api: LocalOfficerApi

const meta = {
    title: "Admin/Signing/LocalNomineePresence",
    component: LocalNomineePresence,
    args: {onGold: fn(async () => undefined)},
    beforeEach: () => {
        api = {
            nomineeStatus: fn(async () => "gold-required" as const),
            nomineeResume: fn(async () => undefined),
            presence: fn(async () => undefined),
            pending: fn(async () => []),
            review: fn(async () => {
                throw new Error("Unexpected officer review in a nominee story")
            }),
            attach: fn(async () => {
                throw new Error("Unexpected attachment in a nominee story")
            }),
        }
    },
    render: ({onGold}) => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <LocalNomineePresence api={api} request={EVENT_ID} digest={DIGEST} onGold={onGold} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ResumeRequiresExplicitGoldStepUp: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const verify = await canvas.findByRole("button", {name: "Verify with Gold to resume"})
        await expect(api.nomineeStatus).toHaveBeenCalledWith(EVENT_ID, DIGEST)
        expect(args.onGold).not.toHaveBeenCalled()
        expect(api.nomineeResume).not.toHaveBeenCalled()
        expect(api.presence).not.toHaveBeenCalled()

        await userEvent.click(verify)
        await waitFor(() => expect(args.onGold).toHaveBeenCalledTimes(1))
        expect(api.nomineeResume).not.toHaveBeenCalled()
        expect(api.presence).not.toHaveBeenCalled()
    },
}
