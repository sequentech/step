// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETemplateType} from "@/types/templates"
import {TemplateEdit} from "./TemplateEdit"
import {
    CREDENTIALS_TEMPLATE_ID,
    TEMPLATE_RESOURCE,
    templateRecords,
} from "./__stories__/TemplateFixture"

interface Scenario {
    /** What reading the template does. */
    reads: ReadState
    /** Whether saving the template fails. */
    failure: boolean
    close: Mock<() => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Template/TemplateEdit",
    component: TemplateEdit,
    args: {reads: "records", failure: false, close: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TEMPLATE_RESOURCE]: templateRecords()}, {reads: args.reads})
        graphql = graphqlBoundary(
            {
                UpdateTemplate: () => {
                    if (args.failure) throw new Error("Synthetic template service unavailable")
                    return {
                        data: {
                            update_sequent_backend_template_by_pk: {id: CREDENTIALS_TEMPLATE_ID},
                        },
                    }
                },
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <TemplateEdit id={CREDENTIALS_TEMPLATE_ID} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const textbox = (canvasElement: HTMLElement, key: string) =>
    within(canvasElement).getByRole("textbox", {name: new RegExp(i18n.t(`template.form.${key}`))})

async function loaded(canvasElement: HTMLElement) {
    await waitFor(() => expect(textbox(canvasElement, "alias")).toHaveValue("voter-credentials"))
}

async function renameTheTemplate(canvasElement: HTMLElement) {
    await loaded(canvasElement)
    await userEvent.type(textbox(canvasElement, "name"), " by SMS")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        await expect(textbox(canvasElement, "name")).toHaveValue("Voter credentials")
        await expect(within(canvasElement).getByText(i18n.t("template.edit.title"))).toBeVisible()
        // Nothing to save until something changes.
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", TEMPLATE_RESOURCE],
        ])
        expect(graphql.calls).toEqual([])
    },
}

export const RenameATemplate: Story = {
    play: async ({canvasElement, args}) => {
        await renameTheTemplate(canvasElement)
        const message = await within(document.body).findByText(i18n.t("template.update.success"))
        await waitFor(() => expect(message).toBeVisible())
        expect(args.close).toHaveBeenCalledOnce()
        expect(graphql.calls).toEqual([
            {
                name: "UpdateTemplate",
                variables: {
                    id: CREDENTIALS_TEMPLATE_ID,
                    tenantId: TENANT_ID,
                    set: expect.objectContaining({
                        alias: "voter-credentials",
                        tenant_id: TENANT_ID,
                        type: ETemplateType.CREDENTIALS,
                        template: expect.objectContaining({
                            alias: "voter-credentials",
                            name: "Voter credentials by SMS",
                            sms: {message: "Vote at {{vote_url}}"},
                        }),
                    }),
                },
                headers: {},
            },
        ])
        // Templates are saved through GraphQL, not the data provider.
        expect(data.writes).toEqual([])
    },
}

export const SaveFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await renameTheTemplate(canvasElement)
        await waitFor(() => expect(graphql.calls.map(({name}) => name)).toEqual(["UpdateTemplate"]))
        const message = await within(document.body).findByText(i18n.t("template.update.error"))
        await waitFor(() => expect(message).toBeVisible())
        // The drawer stays open with what was typed.
        expect(args.close).not.toHaveBeenCalled()
        await expect(textbox(canvasElement, "name")).toHaveValue("Voter credentials by SMS")
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        // The form shows at once, empty until the template arrives.
        await expect(textbox(canvasElement, "alias")).toHaveValue("")
        await expect(within(canvasElement).getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement, args}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        expect(args.close).not.toHaveBeenCalled()
    },
}
