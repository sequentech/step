// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {atViewport} from "../../../../test-support/storybook/viewport"
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider, SimpleForm} from "react-admin"
import {EBallotIdentity, ENetworkDataPolicy} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {PrivacySettings} from "./PrivacySettings"

interface Scenario {
    canEdit: boolean
    network: ENetworkDataPolicy
    legacy: boolean
    failed: boolean
}
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Election event/Privacy settings",
    component: PrivacySettings,
    args: {canEdit: true, network: ENetworkDataPolicy.FORENSIC_STORE, legacy: false, failed: false},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                BallotPrivacySettings: () => {
                    if (args.failed) throw new Error("Synthetic settings failure")
                    return {
                        data: {
                            ballot_privacy_settings: {
                                payload: {
                                    system_public_key:
                                        "MCowBQYDK2VwAyEAiojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w=",
                                    roles: {
                                        forensic_read: ["Election reviewers"],
                                        pseudonym_resolve: [],
                                    },
                                },
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <RecordContextProvider
                value={eventRecord(undefined, {
                    presentation: {
                        ...eventRecord().presentation,
                        network_data_policy: args.network,
                        ballot_identity: args.legacy
                            ? EBallotIdentity.VOTER_ID
                            : EBallotIdentity.PSEUDONYM,
                    },
                })}
            >
                <SimpleForm toolbar={false}>
                    <PrivacySettings canEdit={args.canEdit} />
                </SimpleForm>
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const ForensicStore: Story = {
    ...atViewport(1440),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Election reviewers")).toBeVisible()
        expect(canvas.getByText("No role yet")).toBeVisible()
        expect(canvas.getByRole("radio", {name: "Keep for forensic review"})).toBeChecked()
        expect(canvasElement.scrollWidth).toBeLessThanOrEqual(canvasElement.clientWidth + 1)
    },
}
export const SecondOrganization: Story = {args: {network: ENetworkDataPolicy.DO_NOT_STORE}}
export const Legacy: Story = {
    args: {legacy: true, canEdit: true, network: ENetworkDataPolicy.DO_NOT_STORE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.getByRole("radio", {name: "Keep for forensic review"})).toBeDisabled()
        expect(canvas.getByText(/Forensic attribution is available for new events/)).toBeVisible()
    },
}
export const LoadError: Story = {
    args: {failed: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("The current privacy permissions could not be loaded.")
        ).toBeVisible()
        expect(canvas.queryByText("No role yet")).toBeNull()
    },
}

export const ForensicStorePhone: Story = {
    ...ForensicStore,
    ...atViewport(390),
}

export const ForensicStoreTablet: Story = {
    ...ForensicStore,
    ...atViewport(820),
}

export const LegacyPhone: Story = {
    ...Legacy,
    ...atViewport(390),
}

export const LegacyTablet: Story = {
    ...Legacy,
    ...atViewport(820),
}

export const LoadErrorPhone: Story = {
    ...LoadError,
    ...atViewport(390),
}

export const LoadErrorTablet: Story = {
    ...LoadError,
    ...atViewport(820),
}
