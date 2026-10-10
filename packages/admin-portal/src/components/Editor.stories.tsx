// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Editor} from "tinymce"
import MyEditor from "./Editor"
import {TINYMCE_DEFECTS, editorBody, expectLocalTinymce} from "./__stories__/EditorFixture"

/** The editor instance MyEditor stores on initialisation; each story starts without one. */
const editorRef: {current: Editor | null} = {current: null}

const meta = {
    title: "Admin/Components/MyEditor",
    component: MyEditor,
    args: {
        initialValue: "<p>Dear voter, your ballot is ready.</p>",
        editorRef,
        onEditorChange: fn(),
    },
    argTypes: {editorRef: {table: {disable: true}}},
    parameters: {expectedFailure: TINYMCE_DEFECTS},
    beforeEach: () => {
        editorRef.current = null
    },
    render: (args) => <MyEditor {...args} editorRef={editorRef} />,
} satisfies Meta<typeof MyEditor>
export default meta
type Story = StoryObj<typeof meta>

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const body = await editorBody(canvasElement)
        await expect(within(body).getByText("Dear voter, your ballot is ready.")).toBeVisible()
        expect(editorRef.current?.getContent()).toBe("<p>Dear voter, your ballot is ready.</p>")
        expectLocalTinymce()
        expect(args.onEditorChange).not.toHaveBeenCalled()
    },
}

export const Empty: Story = {
    args: {initialValue: undefined},
    play: async ({canvasElement}) => {
        await editorBody(canvasElement)
        expect(editorRef.current?.getContent()).toBe("")
    },
}

export const TypingReportsTheChange: Story = {
    play: async ({canvasElement, args}) => {
        const body = await editorBody(canvasElement)
        // The direct API types into the editing frame's own document.
        await userEvent.type(
            within(body).getByText("Dear voter, your ballot is ready."),
            " Vote before Friday."
        )
        await waitFor(() => expect(args.onEditorChange).toHaveBeenCalled())
        expect(editorRef.current?.getContent()).toBe(
            "<p>Dear voter, your ballot is ready. Vote before Friday.</p>"
        )
    },
}

export const ControlledValue: Story = {
    args: {initialValue: undefined, value: "<p>Your receipt is attached.</p>"},
    play: async ({canvasElement}) => {
        const body = await editorBody(canvasElement)
        await expect(within(body).getByText("Your receipt is attached.")).toBeVisible()
        expect(editorRef.current?.getContent()).toBe("<p>Your receipt is attached.</p>")
    },
}
