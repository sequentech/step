// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {ScenarioId} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {AuthContext} from "../providers/AuthContextProvider"
import {PreviewScreen} from "../preview/screens"
import {PortalChrome} from "./PortalChrome"

const changeContact = fn(async () => undefined)
const LABEL = "Change how we contact you"
function Fixture() {
    const auth = useContext(AuthContext)
    return (
        <AuthContext.Provider
            value={{
                ...auth,
                isAuthenticated: true,
                contactChangeAvailable: true,
                changeContactChannel: changeContact,
            }}
        >
            <PortalChrome>
                <h1>Choose an election</h1>
            </PortalChrome>
        </AuthContext.Provider>
    )
}
const meta = {
    title: "Voting/Portal chrome/Contact action",
    component: PortalChrome,
    parameters: {
        voterPreview: {scenario: ScenarioId.SIMPLE_PLURALITY, screen: PreviewScreen.START},
    },
    loaders: [() => initCore()],
    beforeEach: () => {
        changeContact.mockClear()
    },
    render: () => <Fixture />,
} satisfies Meta<typeof PortalChrome>
export default meta
type Story = StoryObj<typeof meta>
export const Desktop: Story = {
    play: async ({canvasElement}) => {
        const button = within(canvasElement).getByRole("button", {name: LABEL})
        expect(getComputedStyle(button).backgroundColor).toBe("rgba(0, 0, 0, 0)")
        await userEvent.click(button)
        expect(changeContact).toHaveBeenCalledTimes(1)
    },
}
export const Phone: Story = {
    globals: {viewport: {value: "mobile1", isRotated: false}},
    play: async ({canvasElement}) => {
        const button = within(canvasElement).getByRole("button", {name: LABEL})
        const icon = button.querySelector("svg")
        if (!icon) throw new Error("Contact icon is missing")
        const bounds = button.getBoundingClientRect()
        const glyph = icon.getBoundingClientRect()
        expect(bounds.width).toBeGreaterThanOrEqual(44)
        expect(bounds.width).toBeLessThanOrEqual(48)
        expect(Math.abs(glyph.x + glyph.width / 2 - bounds.x - bounds.width / 2)).toBeLessThan(1)
        await userEvent.hover(button)
        await expect(
            await within(canvasElement.ownerDocument.body).findByRole("tooltip")
        ).toHaveTextContent(LABEL)
        await userEvent.click(button)
        expect(changeContact).toHaveBeenCalledTimes(1)
    },
}
