// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import EmailEditEditor from "./EmailEditEditor"
import {TINYMCE_DEFECTS, editorBody, expectLocalTinymce} from "./__stories__/EditorFixture"

interface Scenario {
    /** The template's parts that the editor offers, as in the template form. */
    parts: "email" | "document"
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
// The host form's toolbar; the widget has no actions of its own.
const saveOnly = (
    <Toolbar>
        <SaveButton />
    </Toolbar>
)

const template = {
    id: 1,
    template: {
        email: {
            subject: "Your ballot is ready",
            html_body: "<p>Dear voter, vote before Friday.</p>",
            plaintext_body: "Dear voter, vote before Friday.",
        },
        document: "<h1>{{election_name}}</h1>",
    },
}

const meta = {
    title: "Admin/Components/EmailEditEditor",
    component: EmailEditEditor,
    args: {parts: "email", onSubmit: fn()},
    argTypes: {parts: {control: "inline-radio", options: ["email", "document"]}},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({parts, onSubmit}) => (
        <AdminStoryProvider boundary={graphql}>
            <SimpleForm record={template} onSubmit={onSubmit} toolbar={saveOnly}>
                {parts === "email" ? (
                    <EmailEditEditor
                        sourceSubject="template.email.subject"
                        sourceBodyHTML="template.email.html_body"
                        sourceBodyPlainText="template.email.plaintext_body"
                    />
                ) : (
                    <EmailEditEditor sourceBodyPlainText="template.document" />
                )}
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tab = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("tab", {name: i18n.t(`emailEditor.tabs.${key}`)})

export const Populated: Story = {
    parameters: {expectedFailure: TINYMCE_DEFECTS, widgets: ["CustomRichTextEditor"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("textbox", {name: i18n.t("emailEditor.subject")})
        ).toHaveValue("Your ballot is ready")
        await expect(tab(canvasElement, "richtext")).toHaveAttribute("aria-selected", "true")
        const body = await editorBody(canvasElement)
        await expect(within(body).getByText("Dear voter, vote before Friday.")).toBeVisible()
        expectLocalTinymce()
    },
}

export const SavesEveryPart: Story = {
    // The rich text tab, and TinyMCE with it, has closed when axe runs.
    parameters: {widgets: ["CustomRichTextEditor"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const body = await editorBody(canvasElement)
        // The direct API types into the editing frame's own document.
        await userEvent.type(within(body).getByText("Dear voter, vote before Friday."), " Thanks!")
        await userEvent.type(
            canvas.getByRole("textbox", {name: i18n.t("emailEditor.subject")}),
            " today"
        )
        await userEvent.click(tab(canvasElement, "plaintext"))
        const plainText = await canvas.findByRole("textbox", {
            name: i18n.t("emailEditor.tabs.plaintext"),
        })
        await userEvent.clear(plainText)
        await userEvent.type(plainText, "Polls close at 20:00.")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit.mock.calls[0][0]).toEqual({
            ...template,
            template: {
                ...template.template,
                email: {
                    subject: "Your ballot is ready today",
                    html_body: "<p>Dear voter, vote before Friday. Thanks!</p>",
                    plaintext_body: "Polls close at 20:00.",
                },
            },
        })
    },
}

export const DocumentTemplate: Story = {
    args: {parts: "document"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        // Without a subject or rich text part, only the plain text tab remains.
        expect(canvas.getAllByRole("tab").map((element) => element.textContent)).toEqual([
            i18n.t("emailEditor.tabs.plaintext"),
        ])
        expect(canvas.queryByRole("textbox", {name: i18n.t("emailEditor.subject")})).toBeNull()
        const document = canvas.getByRole("textbox", {name: i18n.t("emailEditor.tabs.plaintext")})
        await expect(document).toHaveValue("<h1>{{election_name}}</h1>")
        await userEvent.type(document, "<p>Turnout</p>")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit.mock.calls[0][0]).toMatchObject({
            template: {document: "<h1>{{election_name}}</h1><p>Turnout</p>"},
        })
    },
}
