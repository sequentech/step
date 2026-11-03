// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    STORY_SETTINGS,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import {VoterPortalLinks} from "./VoterPortalLinks"

interface Scenario {
    /** Whether the event offers kiosk voting. */
    kiosk: boolean
    /** The deployment's kiosk portal address; empty falls back to the voting portal. */
    kioskPortalUrl?: string
}

let graphql: ReturnType<typeof graphqlBoundary>

function Fixture({kiosk, kioskPortalUrl}: Scenario) {
    const event = eventRecord(undefined, {
        voting_channels: {online: true, kiosk, early_voting: false, telephone: false},
    })
    return (
        <AdminStoryProvider
            boundary={graphql}
            settings={
                kioskPortalUrl === undefined ? undefined : {KIOSK_VOTING_PORTAL_URL: kioskPortalUrl}
            }
        >
            <VoterPortalLinks electionEvent={event} />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Dashboard/Election event/VoterPortalLinks",
    component: VoterPortalLinks,
    args: {kiosk: false},
    beforeEach: async () => {
        // The links come from sequent-core.
        await initCore()
        graphql = graphqlBoundary({})
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const portal = STORY_SETTINGS.VOTING_PORTAL_URL
const eventPath = `/tenant/${TENANT_ID}/event/${STORY_IDS.event}`

export const Online: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("link", {name: "Voter Login URL"})).toHaveAttribute(
            "href",
            `${portal}${eventPath}/login`
        )
        await expect(canvas.getByRole("link", {name: "Voter Enroll URL"})).toHaveAttribute(
            "href",
            `${portal}${eventPath}/enroll`
        )
        expect(canvas.queryByRole("link", {name: "Voter Enroll Kiosk URL"})).toBeNull()
        for (const link of canvas.getAllByRole("link")) {
            await expect(link).toHaveAttribute("target", "_blank")
            await expect(link).toHaveAttribute("rel", "noopener noreferrer")
        }
    },
}

export const KioskEnrollLink: Story = {
    args: {kiosk: true},
    play: async ({canvasElement}) => {
        const link = await within(canvasElement).findByRole("link", {
            name: "Voter Enroll Kiosk URL",
        })
        await expect(link).toHaveAttribute(
            "href",
            `${STORY_SETTINGS.KIOSK_VOTING_PORTAL_URL}${eventPath}/login?kiosk`
        )
        await expect(link).toHaveAttribute("target", "_blank")
        await expect(link).toHaveAttribute("rel", "noopener noreferrer")
    },
}

export const KioskOnTheVotingPortal: Story = {
    args: {kiosk: true, kioskPortalUrl: " "},
    play: async ({canvasElement}) => {
        const link = await within(canvasElement).findByRole("link", {
            name: "Voter Enroll Kiosk URL",
        })
        await expect(link).toHaveAttribute("href", `${portal}${eventPath}/login?kiosk`)
    },
}
