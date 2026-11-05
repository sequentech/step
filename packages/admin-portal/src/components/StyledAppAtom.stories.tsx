// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Typography} from "@mui/material"
import {createStore, Provider as AtomProvider} from "jotai"
import cssInputLookAndFeel from "@/atoms/css-input-look-and-feel"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {StyledAppAtom} from "./StyledAppAtom"
import {
    EStoryTenant,
    STORY_BRANDING,
    useStoryGlobals,
} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Look-and-feel CSS in the shared atom; by default the tenant global's. */
    css?: string
}

function Fixture({css}: Scenario) {
    const {tenant} = useStoryGlobals()
    const [store] = useState(() => {
        const atoms = createStore()
        atoms.set(cssInputLookAndFeel, css ?? STORY_BRANDING[tenant].css ?? "")
        return atoms
    })
    return (
        <AtomProvider store={store}>
            <StyledAppAtom>
                <Typography variant="h4" component="h1">
                    Council event
                </Typography>
                <p className="notice">Voting closes at 20:00.</p>
            </StyledAppAtom>
        </AtomProvider>
    )
}

const meta = {
    title: "Admin/Components/StyledAppAtom",
    component: StyledAppAtom,
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const headingColor = (canvasElement: HTMLElement) =>
    getComputedStyle(within(canvasElement).getByRole("heading", {name: "Council event"})).color

export const FollowsTenantBranding: Story = {
    play: async ({canvasElement, globals}) => {
        const custom = globals.tenant === EStoryTenant.CUSTOM
        expect(headingColor(canvasElement) === "rgb(11, 79, 58)").toBe(custom)
    },
}

export const CustomTenantCss: Story = {
    globals: {tenant: EStoryTenant.CUSTOM},
    play: async ({canvasElement}) => {
        expect(headingColor(canvasElement)).toBe("rgb(11, 79, 58)")
    },
}

export const ScopedToItsChildren: Story = {
    args: {css: ".notice { text-decoration: underline; }"},
    play: async ({canvasElement}) => {
        const notice = within(canvasElement).getByText("Voting closes at 20:00.")
        expect(getComputedStyle(notice).textDecorationLine).toBe("underline")
        expect(notice.closest(".styled-app-atom")).not.toBeNull()
    },
}
