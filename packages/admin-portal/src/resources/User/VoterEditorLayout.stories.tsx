// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {TextField} from "@mui/material"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterEditorRoot, type VoterEditorMode} from "./VoterEditorLayout"

interface Scenario {
    mode: VoterEditorMode
    /** The tenant's voter editor CSS. */
    customCss: string
}

const meta = {
    title: "Admin/User/VoterEditorRoot",
    component: VoterEditorRoot,
    args: {mode: "edit", customCss: ""},
    argTypes: {mode: {control: "inline-radio", options: ["create", "edit"]}},
    render: ({mode, customCss}) => (
        <VoterEditorRoot mode={mode} customCss={customCss}>
            <TextField label="Username" defaultValue="alice" />
        </VoterEditorRoot>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const editor = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: "Username"}).closest(".voter-editor")

export const EditMode: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("textbox", {name: "Username"})).toHaveValue(
            "alice"
        )
        expect(editor(canvasElement)).toHaveAttribute("data-mode", "edit")
    },
}

export const CreateModeWithTenantCss: Story = {
    args: {mode: "create", customCss: ".voter-editor[data-mode='create'] { padding: 24px; }"},
    play: async ({canvasElement}) => {
        const root = editor(canvasElement) as HTMLElement
        expect(root).toHaveAttribute("data-mode", "create")
        expect(getComputedStyle(root).paddingTop).toBe("24px")
    },
}
