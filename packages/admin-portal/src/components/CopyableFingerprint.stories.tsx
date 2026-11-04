// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, spyOn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CopyableFingerprint} from "./CopyableFingerprint"

interface Scenario {
    value: string
    short?: string
    /** What writing to the clipboard does. */
    clipboard: "granted" | "denied"
}

const FULL =
    "F7:08:0F:24:AA:55:BB:66:CC:77:DD:88:EE:99:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:FE:78"
const SHORT = "F7:08:0F:24:…:FE:78"
let boundary: ReturnType<typeof graphqlBoundary>
let copied: string[]

const meta = {
    title: "Admin/Components/CopyableFingerprint",
    component: CopyableFingerprint,
    args: {value: FULL, short: SHORT, clipboard: "granted"},
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
            <CopyableFingerprint {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const copyButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: i18n.t("copyableFingerprint.copy")})

/** Waits for react-admin's notification to finish its entrance transition. */
async function notified(text: string) {
    const message = await within(document.body).findByText(text)
    await waitFor(() => expect(message).toBeVisible())
}

export const TooltipWithTheCompleteValue: Story = {
    play: async ({canvasElement}) => {
        await userEvent.hover(within(canvasElement).getByText(SHORT))
        const tooltip = await within(document.body).findByRole("tooltip")
        await waitFor(() => expect(tooltip).toHaveTextContent(FULL))
    },
}

export const CopiesTheCompleteValue: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(copyButton(canvasElement))
        await waitFor(() => expect(copied).toEqual([FULL]))
        await notified(i18n.t("copyableFingerprint.copied"))
    },
}

export const CopyRejectedByTheBrowser: Story = {
    args: {clipboard: "denied"},
    play: async ({canvasElement}) => {
        await userEvent.click(copyButton(canvasElement))
        await notified(i18n.t("copyableFingerprint.copyError"))
        expect(copied).toEqual([])
    },
}
