// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETemplateType} from "@/types/templates"
import SelectTemplate from "./SelectTemplate"
import {TEMPLATE_RESOURCE, templateRecords} from "./__stories__/TemplateFixture"

interface Scenario {
    /** What reading the templates does. */
    reads: ReadState
    templateType: ETemplateType
    /** The alias the form already has. */
    value?: string
    isRequired: boolean
    disabled: boolean
    /** Whether the tenant has more credentials templates, stored out of name order. */
    moreTemplates: boolean
    onSelectTemplate: Mock<(template: {alias: string}) => void>
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Template/SelectTemplate",
    component: SelectTemplate,
    args: {
        reads: "records",
        templateType: ETemplateType.CREDENTIALS,
        isRequired: false,
        disabled: false,
        moreTemplates: false,
        onSelectTemplate: fn(),
        onSubmit: fn(),
    },
    argTypes: {
        reads: {control: "inline-radio", options: ["records", "loading", "error"]},
        templateType: {control: "select", options: Object.values(ETemplateType)},
    },
    beforeEach: async ({args}) => {
        const templates = templateRecords()
        if (args.moreTemplates) {
            const [credentials] = templates
            templates.unshift(
                {
                    ...credentials,
                    id: storyId(7, 9),
                    alias: "reminder-credentials",
                    template: {...credentials.template, name: "Reminder credentials"},
                },
                {
                    ...credentials,
                    id: storyId(7, 8),
                    alias: "admin-credentials",
                    template: {...credentials.template, name: "Admin credentials"},
                }
            )
        }
        data = resourceBoundary({[TEMPLATE_RESOURCE]: templates}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({templateType, value, isRequired, disabled, onSelectTemplate, onSubmit}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SimpleForm
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton alwaysEnable />
                    </Toolbar>
                }
            >
                <SelectTemplate
                    tenantId={TENANT_ID}
                    templateType={templateType}
                    source="template_alias"
                    label="Credentials template"
                    value={value}
                    isRequired={isRequired}
                    disabled={disabled}
                    onSelectTemplate={onSelectTemplate}
                />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const templateInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {name: /Credentials template/})

async function openOptions(canvasElement: HTMLElement) {
    await userEvent.click(templateInput(canvasElement))
    const listbox = await within(document.body).findByRole("listbox")
    return within(listbox)
        .getAllByRole("option")
        .map((option) => option.textContent)
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls).toEqual([
                {
                    method: "getList",
                    args: [
                        TEMPLATE_RESOURCE,
                        expect.objectContaining({
                            filter: {tenant_id: TENANT_ID, type: ETemplateType.CREDENTIALS},
                            sort: {field: "template.name", order: "ASC"},
                            pagination: {page: 1, perPage: 100},
                        }),
                    ],
                },
            ])
        )
        // Only the templates of the requested type are offered.
        await waitFor(async () =>
            expect(await openOptions(canvasElement)).toEqual(["Voter credentials"])
        )
    },
}

export const SortedByName: Story = {
    args: {moreTemplates: true},
    play: async ({canvasElement}) => {
        await waitFor(async () =>
            expect(await openOptions(canvasElement)).toEqual([
                "Admin credentials",
                "Reminder credentials",
                "Voter credentials",
            ])
        )
    },
}

export const ChooseATemplate: Story = {
    play: async ({canvasElement, args}) => {
        await waitFor(async () => expect(await openOptions(canvasElement)).toHaveLength(1))
        await userEvent.click(
            within(document.body).getByRole("option", {name: "Voter credentials"})
        )
        expect(args.onSelectTemplate).toHaveBeenCalledWith({alias: "voter-credentials"})
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit.mock.calls[0][0]).toEqual({template_alias: "voter-credentials"})
    },
}

export const SavedTemplate: Story = {
    args: {value: "voter-credentials"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(templateInput(canvasElement)).toHaveValue("Voter credentials"))
    },
}

export const RequiredTemplate: Story = {
    args: {isRequired: true},
    play: async ({canvasElement, args}) => {
        await waitFor(() => expect(data.calls).toHaveLength(1))
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await expect(await within(canvasElement).findByText("Required")).toBeVisible()
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}

export const Disabled: Story = {
    args: {disabled: true, value: "voter-credentials"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(templateInput(canvasElement)).toHaveValue("Voter credentials"))
        await expect(templateInput(canvasElement)).toBeDisabled()
    },
}

export const NoTemplatesOfTheType: Story = {
    args: {templateType: ETemplateType.ACTIVITY_LOGS},
    play: async ({canvasElement, args}) => {
        await waitFor(() => expect(data.calls).toHaveLength(1))
        await userEvent.click(templateInput(canvasElement))
        await expect(await within(document.body).findByText("No options")).toBeVisible()
        expect(args.onSelectTemplate).not.toHaveBeenCalled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls).toHaveLength(1))
        await expect(templateInput(canvasElement)).toHaveValue("")
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls).toHaveLength(1))
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        await userEvent.click(templateInput(canvasElement))
        await expect(await within(document.body).findByText("No options")).toBeVisible()
    },
}
