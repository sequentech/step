// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import type {IEmail} from "@/types/templates"
import EmailEditor from "./EmailEditor"
import {TINYMCE_DEFECTS, editorBody, expectLocalTinymce} from "./__stories__/EditorFixture"

const email: IEmail = {
    subject: "Your ballot is ready",
    plaintext_body: "Dear voter, vote before Friday.",
    html_body: "<p>Dear voter, <strong>vote before Friday</strong>.</p>",
}

/** SendTemplate keeps the email being edited in its state. */
function EmailHost({record: initial, setRecord}: React.ComponentProps<typeof EmailEditor>) {
    const [record, setState] = useState(initial)
    return (
        <EmailEditor
            record={record}
            setRecord={(next) => {
                setRecord(next)
                setState(next)
            }}
        />
    )
}

const meta = {
    title: "Admin/Components/EmailEditor",
    component: EmailEditor,
    args: {record: email, setRecord: fn()},
    render: (args) => <EmailHost {...args} />,
} satisfies Meta<typeof EmailEditor>
export default meta
type Story = StoryObj<typeof meta>

const subject = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: i18n.t("emailEditor.subject")})
const plainText = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: i18n.t("emailEditor.tabs.plaintext")})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(subject(canvasElement)).toHaveValue("Your ballot is ready")
        await expect(
            canvas.getByRole("tab", {name: i18n.t("emailEditor.tabs.plaintext")})
        ).toHaveAttribute("aria-selected", "true")
        await expect(plainText(canvasElement)).toHaveValue("Dear voter, vote before Friday.")
    },
}

export const EditsSubjectAndPlainText: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.type(subject(canvasElement), " today")
        await userEvent.clear(plainText(canvasElement))
        await userEvent.type(plainText(canvasElement), "Polls close at 20:00.")
        expect(args.setRecord).toHaveBeenLastCalledWith({
            ...email,
            subject: "Your ballot is ready today",
            plaintext_body: "Polls close at 20:00.",
        })
    },
}

export const EditsRichText: Story = {
    parameters: {expectedFailure: TINYMCE_DEFECTS},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("tab", {name: i18n.t("emailEditor.tabs.richtext")}))
        const body = await editorBody(canvasElement)
        expectLocalTinymce()
        const paragraph = within(body).getByText(/Dear voter/)
        await expect(within(paragraph).getByText("vote before Friday")).toBeVisible()
        // The direct API types into the editing frame's own document. One key:
        // each change comes back as the editor's initialValue, which resets it.
        await userEvent.type(paragraph, "!")
        await waitFor(() =>
            expect(args.setRecord).toHaveBeenLastCalledWith({
                ...email,
                html_body: "<p>Dear voter, <strong>vote before Friday</strong>.!</p>",
            })
        )
    },
}
