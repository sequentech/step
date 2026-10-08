// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETemplateType, ITemplateMethod} from "@/types/templates"
import {TemplateCreate} from "./TemplateCreate"
import {TEMPLATE_RESOURCE} from "./__stories__/TemplateFixture"

interface Scenario {
    /** Whether saving the template fails. */
    failure: boolean
    close: Mock<() => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Template/TemplateCreate",
    component: TemplateCreate,
    args: {failure: false, close: fn()},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TEMPLATE_RESOURCE]: []})
        graphql = graphqlBoundary(
            {
                GetUserTemplate: () => ({
                    data: {
                        get_user_template: {
                            template_hbs: "<p>{{vote_url}}</p>",
                            extra_config: JSON.stringify({
                                communication_templates: {
                                    email_config: {
                                        subject: "Your credentials",
                                        plaintext_body: "{{vote_url}}",
                                        html_body: "<p>{{vote_url}}</p>",
                                    },
                                    sms_config: {message: "{{vote_url}}"},
                                },
                            }),
                        },
                    },
                }),
                InsertTemplate: () => {
                    if (args.failure) throw new Error("Synthetic template service unavailable")
                    return {data: {insert_sequent_backend_template: {affected_rows: 1}}}
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <TemplateCreate close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const textbox = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("textbox", {name: new RegExp(i18n.t(`template.form.${key}`))})

async function fillTheTemplate(canvasElement: HTMLElement) {
    await userEvent.type(textbox(canvasElement, "alias"), "council-credentials")
    await userEvent.type(textbox(canvasElement, "name"), "Council credentials")
    await userEvent.click(
        within(canvasElement).getByRole("combobox", {
            name: new RegExp(i18n.t("template.form.type")),
        })
    )
    await userEvent.click(
        await within(document.body).findByRole("option", {
            name: i18n.t(`template.type.${ETemplateType.CREDENTIALS}`),
        })
    )
    await userEvent.click(
        within(canvasElement).getByRole("switch", {name: i18n.t("template.method.sms")})
    )
    await waitFor(() => expect(graphql.calls.map(({name}) => name)).toEqual(["GetUserTemplate"]))
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Empty: Story = {
    play: async ({canvasElement, args}) => {
        await expect(within(canvasElement).getByText(i18n.t("template.create.title"))).toBeVisible()
        await expect(textbox(canvasElement, "alias")).toHaveValue("")
        expect(graphql.calls).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const CreateATemplate: Story = {
    play: async ({canvasElement, args}) => {
        await fillTheTemplate(canvasElement)
        const message = await within(document.body).findByText(i18n.t("template.create.success"))
        await waitFor(() => expect(message).toBeVisible())
        expect(args.close).toHaveBeenCalledOnce()
        const insert = graphql.calls.find(({name}) => name === "InsertTemplate")
        expect(insert?.variables).toEqual({
            object: {
                alias: "council-credentials",
                tenant_id: TENANT_ID,
                type: ETemplateType.CREDENTIALS,
                communication_method: ITemplateMethod.SMS,
                template: expect.objectContaining({
                    alias: "council-credentials",
                    name: "Council credentials",
                    selected_methods: {
                        EMAIL: false,
                        SMS: true,
                        DOCUMENT: false,
                        WHATSAPP: false,
                        VIBER: false,
                        MESSENGER: false,
                    },
                    sms: {message: "{{vote_url}}"},
                    document: "<p>{{vote_url}}</p>",
                }),
            },
        })
        // Templates are saved through GraphQL, not the data provider.
        expect(data.writes).toEqual([])
    },
}

export const CreateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await fillTheTemplate(canvasElement)
        await waitFor(() =>
            expect(graphql.calls.map(({name}) => name)).toEqual([
                "GetUserTemplate",
                "InsertTemplate",
            ])
        )
        const message = await within(document.body).findByText(i18n.t("template.create.error"))
        await waitFor(() => expect(message).toBeVisible())
        // The drawer stays open with what was typed.
        expect(args.close).not.toHaveBeenCalled()
        await expect(textbox(canvasElement, "alias")).toHaveValue("council-credentials")
    },
}
