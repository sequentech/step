// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {IRetainedSignedClose} from "@/types/lifecycle"
import {RetainedSignedCloses} from "./RetainedSignedCloses"
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
    component: RetainedSignedCloses,
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: () => (
        <AdminStoryProvider boundary={boundary}>
            <RetainedSignedCloses
                closes={[{...close, fired_at: "2028-05-08T11:00:04Z"}]}
                zoneOf={() => "Asia/Manila"}
                nameOf={() => "Post"}
                unavailable
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const RetainedListUnavailable: Story = {
    parameters: {widgets: ["RetainedSignedCloseNotice"]},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("lifecycle.signedClose.unavailable"))
        ).toBeVisible()
        await expect(
            within(canvasElement).getByText(i18n.t("lifecycle.signedClose.result"))
        ).toBeVisible()
    },
}
