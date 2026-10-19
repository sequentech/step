// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {EElectionEventLockedDown, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {eventRecord} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"
import {LockedDownDataTab} from "./ElectionEventDataTab"

type Args = {lockedDown: boolean}

/** A locked-down event, or one that is not, for an admin who may check readiness. */
const lockedEvent = (lockedDown: boolean) => {
    const event = eventRecord()
    return {
        ...event,
        presentation: {
            ...(event.presentation ?? {}),
            locked_down: lockedDown
                ? EElectionEventLockedDown.LOCKED_DOWN
                : EElectionEventLockedDown.NOT_LOCKED_DOWN,
        },
    }
}

let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/LockedDownDataTab",
    component: LockedDownDataTab,
    args: {lockedDown: true},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({lockedDown}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={[IPermissions.ELECTION_EVENT_DATA_TAB, IPermissions.READINESS_CHECK]}
        >
            <RecordContextProvider value={lockedEvent(lockedDown)}>
                <LockedDownDataTab />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Args>
export default meta
type Story = StoryObj<Args>

export const LockedDown: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("readiness.action")})
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("form")).toBeNull()
    },
}

export const NotLockedDown: Story = {
    args: {lockedDown: false},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(i18n.t("electionEventScreen.dataUnavailable"))
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("button")).toBeNull()
    },
}
