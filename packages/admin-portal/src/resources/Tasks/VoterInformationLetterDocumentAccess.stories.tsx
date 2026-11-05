// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterInformationLetterDocumentAccess} from "./VoterInformationLetterDocumentAccess"

interface Scenario {
    taskId: string
    pdfPassword?: string
    loading: boolean
    onReveal: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Tasks/VoterInformationLetterDocumentAccess",
    component: VoterInformationLetterDocumentAccess,
    args: {taskId: "99999999-9999-4999-8999-999999999991", loading: false, onReveal: fn()},
    beforeEach: async () => {
        boundary = graphqlBoundary({}, {schema: true})
        await boundary.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <VoterInformationLetterDocumentAccess {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const summary = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: i18n.t("tasksScreen.documentAccess.title")})

export const Expanded: Story = {
    play: async ({canvasElement, args}) => {
        expect(summary(canvasElement)).toHaveAttribute("aria-expanded", "true")
        await userEvent.click(
            within(canvasElement).getByRole("button", {
                name: i18n.t("tasksScreen.documentAccess.showPassword"),
            })
        )
        expect(args.onReveal).toHaveBeenCalledTimes(1)
    },
}

export const Collapse: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(summary(canvasElement))
        await waitFor(() =>
            expect(summary(canvasElement)).toHaveAttribute("aria-expanded", "false")
        )
    },
}

export const PasswordShown: Story = {
    args: {pdfPassword: "Synthetic-PDF-Password-42"},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("textbox", {
                name: i18n.t("tasksScreen.documentAccess.passwordLabel"),
            })
        ).toHaveValue("Synthetic-PDF-Password-42")
    },
}
