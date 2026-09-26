// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {CreateBase, EditBase, SimpleForm} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {IPermissions} from "@/types/keycloak"
import {ETemplateType} from "@/types/templates"
import {TINYMCE_DEFECTS} from "@/components/__stories__/EditorFixture"
import {TemplateFormContent} from "./TemplateFormContent"
import {
    CREDENTIALS_TEMPLATE_ID,
    TEMPLATE_RESOURCE,
    templateRecords,
} from "./__stories__/TemplateFixture"

interface Scenario {
    /** Whether the form edits a saved template. */
    editing: boolean
    /** Whether the default template service answers. */
    defaults: "available" | "failure"
    onFormChanged: Mock<() => void>
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

const RECEIPT_DEFAULT = "<h1>Default receipt {{ballot_id}}</h1>"

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const OPEN_ACCORDIONS = {
    reason: "The open method accordions are regions without distinct names.",
    a11y: ["landmark-unique"],
}

const meta = {
    title: "Admin/Template/TemplateFormContent",
    component: TemplateFormContent,
    args: {editing: false, defaults: "available", onFormChanged: fn(), onSubmit: fn()},
    argTypes: {defaults: {control: "inline-radio", options: ["available", "failure"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TEMPLATE_RESOURCE]: templateRecords()})
        graphql = graphqlBoundary(
            {
                GetUserTemplate: () => {
                    if (args.defaults === "failure") {
                        throw new Error("Synthetic template service unavailable")
                    }
                    return {
                        data: {
                            get_user_template: {
                                template_hbs: RECEIPT_DEFAULT,
                                extra_config: JSON.stringify({
                                    pdf_options: {format: "Letter"},
                                    report_options: {max_items_per_report: 50},
                                    communication_templates: {
                                        email_config: {
                                            subject: "Your receipt",
                                            plaintext_body: "Receipt",
                                            html_body: "<p>Receipt</p>",
                                        },
                                        sms_config: {message: "Receipt"},
                                    },
                                }),
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({editing, onFormChanged, onSubmit}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            {editing ? (
                <EditBase
                    resource={TEMPLATE_RESOURCE}
                    id={CREDENTIALS_TEMPLATE_ID}
                    redirect={false}
                >
                    <SimpleForm onSubmit={onSubmit}>
                        <TemplateFormContent isTemplateEdit onFormChanged={onFormChanged} />
                    </SimpleForm>
                </EditBase>
            ) : (
                <CreateBase resource={TEMPLATE_RESOURCE} redirect={false}>
                    <SimpleForm onSubmit={onSubmit}>
                        <TemplateFormContent isTemplateEdit={false} onFormChanged={onFormChanged} />
                    </SimpleForm>
                </CreateBase>
            )}
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const textbox = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("textbox", {name: new RegExp(i18n.t(`template.form.${key}`))})

const methodSwitch = (canvasElement: HTMLElement, method: string) =>
    within(canvasElement).getByRole("switch", {name: i18n.t(`template.method.${method}`)})

const accordion = (canvasElement: HTMLElement, title: string) =>
    within(canvasElement).queryByRole("button", {name: title})

async function chooseType(canvasElement: HTMLElement, type: ETemplateType) {
    await userEvent.click(
        within(canvasElement).getByRole("combobox", {
            name: new RegExp(i18n.t("template.form.type")),
        })
    )
    await userEvent.click(
        await within(document.body).findByRole("option", {name: i18n.t(`template.type.${type}`)})
    )
}

async function nameTheReceipt(canvasElement: HTMLElement) {
    await userEvent.type(textbox(canvasElement, "alias"), "council-receipt")
    await userEvent.type(textbox(canvasElement, "name"), "Council receipt")
}

const submitted = (onSubmit: Scenario["onSubmit"]) =>
    onSubmit.mock.calls[0][0] as {template: Record<string, unknown>; type: string}

export const Empty: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("template.create.title"))).toBeVisible()
        await expect(textbox(canvasElement, "alias")).toHaveValue("")
        await expect(textbox(canvasElement, "name")).toHaveValue("")
        for (const method of ["email", "sms", "document"]) {
            await expect(methodSwitch(canvasElement, method)).not.toBeChecked()
        }
        expect(accordion(canvasElement, i18n.t("template.form.smsMessage"))).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const RequiresAliasNameAndType: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(methodSwitch(canvasElement, "sms"))
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() =>
            expect(within(canvasElement).getAllByText("Required").length).toBeGreaterThanOrEqual(3)
        )
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}

export const StartFromTheDefaultTemplate: Story = {
    play: async ({canvasElement, args}) => {
        await nameTheReceipt(canvasElement)
        await chooseType(canvasElement, ETemplateType.BALLOT_RECEIPT)
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "GetUserTemplate",
                    variables: {template_type: "ballot_receipt"},
                    headers: {"x-hasura-role": IPermissions.REPORT_READ},
                },
            ])
        )
        await userEvent.click(methodSwitch(canvasElement, "document"))
        await expect(accordion(canvasElement, i18n.t("template.form.pdfOptions"))).toBeVisible()
        await expect(accordion(canvasElement, i18n.t("template.form.reportOptions"))).toBeVisible()
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        const values = submitted(args.onSubmit)
        expect(values.type).toBe(ETemplateType.BALLOT_RECEIPT)
        expect(values.template).toMatchObject({
            alias: "council-receipt",
            name: "Council receipt",
            selected_methods: {DOCUMENT: true},
            document: RECEIPT_DEFAULT,
            pdf_options: {format: "Letter"},
            report_options: {max_items_per_report: 50},
            sms: {message: "Receipt"},
        })
    },
}

export const DefaultTemplateUnavailable: Story = {
    args: {defaults: "failure"},
    play: async ({canvasElement, args}) => {
        await nameTheReceipt(canvasElement)
        await chooseType(canvasElement, ETemplateType.BALLOT_RECEIPT)
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual(["GetUserTemplate"])
        )
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        // The portal's default document, set before the form was ready, is lost.
        expect(submitted(args.onSubmit).template.document).toBeUndefined()
    },
}

export const ChooseTheMethods: Story = {
    parameters: {
        expectedFailure: {
            reason: `${OPEN_ACCORDIONS.reason} ${TINYMCE_DEFECTS.reason}`,
            a11y: [...TINYMCE_DEFECTS.a11y, ...OPEN_ACCORDIONS.a11y],
        },
    },
    play: async ({canvasElement}) => {
        await userEvent.click(methodSwitch(canvasElement, "email"))
        await userEvent.click(methodSwitch(canvasElement, "sms"))
        await userEvent.click(
            accordion(canvasElement, i18n.t("template.method.email")) as HTMLElement
        )
        await expect(
            await within(canvasElement).findByRole("textbox", {name: i18n.t("emailEditor.subject")})
        ).toBeVisible()
        // The email body's rich text editor has loaded.
        await waitFor(
            () =>
                expect(canvasElement.querySelector(".tox-statusbar__resize-handle")).not.toBeNull(),
            {timeout: 10_000}
        )
        const sms = accordion(canvasElement, i18n.t("template.form.smsMessage")) as HTMLElement
        await userEvent.click(sms)
        await userEvent.type(textbox(canvasElement, "smsMessage"), "Vote now")
        await expect(textbox(canvasElement, "smsMessage")).toHaveValue("Vote now")
        expect(accordion(canvasElement, i18n.t("template.form.pdfOptions"))).toBeNull()
    },
}

export const EditASavedTemplate: Story = {
    args: {editing: true},
    play: async ({canvasElement, args}) => {
        await waitFor(() =>
            expect(textbox(canvasElement, "alias")).toHaveValue("voter-credentials")
        )
        await expect(within(canvasElement).getByText(i18n.t("template.edit.title"))).toBeVisible()
        await expect(methodSwitch(canvasElement, "email")).toBeChecked()
        await expect(methodSwitch(canvasElement, "sms")).toBeChecked()
        await expect(methodSwitch(canvasElement, "document")).not.toBeChecked()
        // The saved template is used as it is.
        expect(graphql.calls).toEqual([])
        await chooseType(canvasElement, ETemplateType.BALLOT_RECEIPT)
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual(["GetUserTemplate"])
        )
        // Back on its own type, the saved template is restored without asking again.
        await chooseType(canvasElement, ETemplateType.CREDENTIALS)
        await userEvent.type(textbox(canvasElement, "name"), " by email")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(graphql.calls).toHaveLength(1)
        expect(submitted(args.onSubmit).template).toMatchObject({
            name: "Voter credentials by email",
            sms: {message: "Vote at {{vote_url}}"},
            email: {subject: "Your voting credentials"},
        })
        expect(args.onFormChanged).not.toHaveBeenCalled()
    },
}
