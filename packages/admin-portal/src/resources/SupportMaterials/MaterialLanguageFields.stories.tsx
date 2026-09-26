// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MaterialLanguageFields} from "./MaterialLanguageFields"

interface Scenario {
    title: string
    subtitle: string
    onTitleChange: Mock<(value: string) => void>
    onSubtitleChange: Mock<(value: string) => void>
    onSubmit: Mock<(values: Record<string, unknown>) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>

const titleLabel = () => i18n.t("electionEventScreen.field.materialTitle")
const subtitleLabel = () => i18n.t("electionEventScreen.field.materialSubTitle")

/** The material forms keep the texts in their own state and require a title. */
function MaterialForm({title, subtitle, onTitleChange, onSubtitleChange, onSubmit}: Scenario) {
    const [titleValue, setTitle] = useState(title)
    const [subtitleValue, setSubtitle] = useState(subtitle)
    return (
        <SimpleForm
            onSubmit={onSubmit}
            validate={() => (titleValue ? {} : {data: i18n.t("materials.error.title")})}
            toolbar={
                <Toolbar>
                    <SaveButton alwaysEnable />
                </Toolbar>
            }
        >
            <MaterialLanguageFields
                titleLabel={titleLabel()}
                subtitleLabel={subtitleLabel()}
                titleValue={titleValue}
                subtitleValue={subtitleValue}
                onTitleChange={(value) => {
                    setTitle(value)
                    onTitleChange(value)
                }}
                onSubtitleChange={(value) => {
                    setSubtitle(value)
                    onSubtitleChange(value)
                }}
            />
        </SimpleForm>
    )
}

const meta = {
    title: "Admin/SupportMaterials/MaterialLanguageFields",
    component: MaterialLanguageFields,
    args: {
        title: "Voting guide",
        subtitle: "How to vote online",
        onTitleChange: fn(),
        onSubtitleChange: fn(),
        onSubmit: fn(),
    },
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <MaterialForm {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const field = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByRole("textbox", {name: label})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(field(canvasElement, titleLabel())).toHaveValue("Voting guide")
        await expect(field(canvasElement, subtitleLabel())).toHaveValue("How to vote online")
    },
}

export const EditTheTexts: Story = {
    args: {title: "", subtitle: ""},
    play: async ({canvasElement, args}) => {
        await userEvent.type(field(canvasElement, titleLabel()), "Guide")
        await userEvent.type(field(canvasElement, subtitleLabel()), "Online")
        expect(args.onTitleChange).toHaveBeenLastCalledWith("Guide")
        expect(args.onSubtitleChange).toHaveBeenLastCalledWith("Online")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
    },
}

export const TitleRequired: Story = {
    args: {title: ""},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
        await expect(
            await within(canvasElement).findByText(i18n.t("materials.error.title"))
        ).toBeVisible()
        await expect(field(canvasElement, titleLabel())).toHaveAttribute("aria-invalid", "true")
        expect(args.onSubmit).not.toHaveBeenCalled()
    },
}
