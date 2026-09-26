// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterInformationLetterPasswordAccess} from "./VoterInformationLetterPasswordAccess"

interface Scenario {
    pdfPassword?: string
    loading: boolean
    /** Absent when the viewer cannot reveal the password. */
    onReveal?: () => void
    /** What writing to the clipboard does. */
    clipboard: "granted" | "denied"
}

const PASSWORD = "Synthetic-PDF-Password-42"
let boundary: ReturnType<typeof graphqlBoundary>
let copied: string[]

const meta = {
    title: "Admin/Tasks/VoterInformationLetterPasswordAccess",
    component: VoterInformationLetterPasswordAccess,
    args: {loading: false, onReveal: fn(), clipboard: "granted"},
    argTypes: {clipboard: {control: "inline-radio", options: ["granted", "denied"]}},
    beforeEach: async ({args}) => {
        copied = []
        boundary = graphqlBoundary({}, {schema: true})
        await boundary.ready
        const clipboard = spyOn(navigator.clipboard, "writeText").mockImplementation(
            async (text) => {
                if (args.clipboard === "denied") throw new DOMException("Denied", "NotAllowedError")
                copied.push(text)
            }
        )
        return () => clipboard.mockRestore()
    },
    render: ({clipboard: _clipboard, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <VoterInformationLetterPasswordAccess {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const showPassword = () => i18n.t("tasksScreen.documentAccess.showPassword")

/** Waits for react-admin's notification to finish its entrance transition. */
async function notified(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

export const PasswordHidden: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("tasksScreen.documentAccess.sensitivityNotice"))
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: showPassword()}))
        expect(args.onReveal).toHaveBeenCalledTimes(1)
        expect(canvas.queryByRole("textbox")).toBeNull()
    },
}

export const LoadingPassword: Story = {
    args: {loading: true},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("button", {name: showPassword()})
        ).toBeDisabled()
    },
}

export const WithoutRevealPermission: Story = {
    args: {onReveal: undefined},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByRole("button", {name: showPassword()})
        ).toBeDisabled()
    },
}

export const PasswordShownAndCopied: Story = {
    args: {pdfPassword: PASSWORD},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const field = canvas.getByRole("textbox", {
            name: i18n.t("tasksScreen.documentAccess.passwordLabel"),
        })
        expect(field).toHaveValue(PASSWORD)
        expect(field).toHaveAttribute("readonly")
        expect(canvas.queryByRole("button", {name: showPassword()})).toBeNull()
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("tasksScreen.documentAccess.copyPassword")})
        )
        await waitFor(() => expect(copied).toEqual([PASSWORD]))
        await notified(i18n.t("tasksScreen.documentAccess.passwordCopied"))
    },
}

export const CopyRejectedByTheBrowser: Story = {
    args: {pdfPassword: PASSWORD, clipboard: "denied"},
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("button", {
                name: i18n.t("tasksScreen.documentAccess.copyPassword"),
            })
        )
        await notified(i18n.t("tasksScreen.documentAccess.copyError"))
        expect(copied).toEqual([])
    },
}
