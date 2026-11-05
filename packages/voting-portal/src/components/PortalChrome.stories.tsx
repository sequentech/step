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
const LABEL = "Contact method"
function Fixture() {
    const auth = useContext(AuthContext)
    return (
        <AuthContext.Provider
            value={{
                ...auth,
                isAuthenticated: true,
                contactChangeAvailable: true,
                changeContactChannel: changeContact,
                firstName: "Demo voter",
                username: "demo-voter",
                email: "voter@example.test",
                openProfileLink: async () => undefined,
                logout: () => undefined,
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
        expect(within(canvasElement).queryByRole("button", {name: LABEL})).toBeNull()
        const trigger = canvasElement.querySelector<HTMLElement>(".profile-menu-button")
        if (!trigger) throw new Error("User menu trigger is missing")
        await userEvent.click(trigger)
        const body = within(canvasElement.ownerDocument.body)
        const menu = await body.findByRole("menu")
        const item = within(menu).getByRole("menuitem", {name: LABEL})
        expect(within(menu).queryByRole("button")).toBeNull()
        expect(menu.querySelector(".user-details")).not.toHaveAttribute("role", "menuitem")
        expect(menu.querySelector(".user-details")).not.toHaveAttribute("tabindex")
        const entries = within(menu).getAllByRole("menuitem")
        const menuColor = getComputedStyle(entries[0]).color
        for (const entry of entries) {
            const icon = entry.querySelector("svg")
            if (!icon) throw new Error("Menu icon is missing")
            expect(getComputedStyle(entry).color).toBe(menuColor)
            expect(getComputedStyle(icon).color).toBe(getComputedStyle(entry).color)
            await userEvent.hover(entry)
            expect(getComputedStyle(icon).color).toBe(menuColor)
            entry.focus()
            expect(getComputedStyle(entry).color).toBe(menuColor)
            expect(getComputedStyle(icon).color).toBe(menuColor)
            await userEvent.unhover(entry)
        }
        await userEvent.click(item)
        expect(changeContact).toHaveBeenCalledTimes(1)
        await expect(body.queryByRole("menu")).toBeNull()
        await userEvent.click(trigger)
        await userEvent.click(body.getByRole("menuitem", {name: "Logout"}))
        await expect(body.getByRole("dialog")).toHaveTextContent("Are you sure you want to logout?")
    },
}
export const Phone: Story = {
    globals: {viewport: {value: "mobile1", isRotated: false}},
    play: async ({canvasElement}) => {
        const trigger = canvasElement.querySelector<HTMLElement>(".profile-menu-button")
        if (!trigger) throw new Error("User menu trigger is missing")
        await userEvent.click(trigger)
        const menu = await within(canvasElement.ownerDocument.body).findByRole("menu")
        const item = within(menu).getByRole("menuitem", {name: LABEL})
        const bounds = menu.getBoundingClientRect()
        expect(bounds.x).toBeGreaterThanOrEqual(0)
        expect(bounds.right).toBeLessThanOrEqual(window.innerWidth)
        item.focus()
        await userEvent.keyboard("{Enter}")
        expect(changeContact).toHaveBeenCalledTimes(1)
    },
}
