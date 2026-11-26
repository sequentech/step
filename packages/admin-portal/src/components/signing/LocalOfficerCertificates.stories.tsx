// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, within} from "storybook/test"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {LocalOfficerCertificates} from "./LocalInitializeEnrollment"
import {MARIA, signedInAs} from "./__stories__/fixtures"

interface Scenario {
    onAttached: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Signing/LocalOfficerCertificates",
    component: LocalOfficerCertificates,
    args: {onAttached: fn()},
    beforeEach: () => {
        boundary = graphqlBoundary({})
    },
    render: ({onAttached}) => (
        <AdminStoryProvider
            boundary={boundary}
            roles={["signing-certificates-register"]}
            auth={signedInAs(MARIA)}
        >
            <div data-testid="ordinary-officer-context">
                <LocalOfficerCertificates event={EVENT_ID} onAttached={onAttached} />
            </div>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const DoesNotMountWithoutRegisteredLocalBuild: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const wrapper = await canvas.findByTestId("ordinary-officer-context")
        expect(within(wrapper).queryByRole("button")).toBeNull()
        expect(wrapper).not.toHaveTextContent("Initialize signing certificate enrollment")
        expect(args.onAttached).not.toHaveBeenCalled()
        expect(boundary.calls).toHaveLength(0)
    },
}
