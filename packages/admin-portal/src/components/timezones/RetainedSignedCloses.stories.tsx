// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {IRetainedSignedClose} from "@/types/lifecycle"
import {RetainedSignedCloseNotice} from "./RetainedSignedCloses"
import {MyTimeZoneProvider} from "./timeZoneService"
import {MY_TIME_ZONE, overseasConfiguration} from "./__fixtures__/configurations"

const post = overseasConfiguration().elections[0]
const close: IRetainedSignedClose = {
    scheduled_event_id: "55555555-5555-4555-8555-000000000021",
    election_id: post.id,
    fingerprint: "signed-close-fixture",
    scheduled_at: "2028-05-08T11:00:00Z",
    channels: ["ONLINE"],
    authorized_by: {
        request_id: "signed-configuration",
        code: "SCHEDULE-2",
        signers: ["Schedule trustee"],
    },
}

interface Scenario {
    processed: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Timezones/RetainedSignedCloses",
    component: RetainedSignedCloseNotice,
    args: {processed: false},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({processed}) => (
        <AdminStoryProvider boundary={boundary}>
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <RetainedSignedCloseNotice
                    close={{...close, fired_at: processed ? "2028-05-08T11:00:04Z" : undefined}}
                    zone={post.timezone ?? "Asia/Manila"}
                    electionName={post.name}
                    now={Date.parse("2028-05-08T10:00:00Z")}
                />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const RetainedAfterScheduleDeletion: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("lifecycle.signedClose.title"))).toBeVisible()
        await expect(canvas.getByText(/SCHEDULE-2/)).toBeVisible()
        await expect(canvas.getByText(i18n.t("lifecycle.signedClose.explanation"))).toBeVisible()
    },
}
export const ProcessedWithoutClaimingClosure: Story = {
    args: {processed: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("lifecycle.signedClose.result"))).toBeVisible()
        await expect(canvas.queryByText(i18n.t("lifecycle.signedClose.title"))).toBeNull()
        await expect(canvas.queryByText(i18n.t("lifecycle.signedClose.explanation"))).toBeNull()
    },
}
