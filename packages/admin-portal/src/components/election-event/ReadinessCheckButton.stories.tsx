// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {eventRecord} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"
import {ReadinessCheckButton} from "./ReadinessCheckButton"

let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/ReadinessCheckButton",
    component: ReadinessCheckButton,
    args: {authorized: true},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({authorized}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={authorized ? [IPermissions.READINESS_CHECK] : []}
        >
            <ReadinessCheckButton event={eventRecord()} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<{authorized: boolean}>
export default meta
type Story = StoryObj<{authorized: boolean}>
export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("readiness.action")})
        ).toBeVisible()
        expect(boundary.calls).toEqual([])
    },
}
export const WithoutPermission: Story = {
    args: {authorized: false},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(boundary.calls).toEqual([])
    },
}
