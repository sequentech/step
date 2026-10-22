// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within, type Mock} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {AudioInstructionsFields, type AudioInstructionsValue} from "./AudioInstructionsFields"

interface Scenario {
    kind: string
    languages: string[]
    value?: AudioInstructionsValue
    onChange: Mock<(value: AudioInstructionsValue | undefined) => void>
}

let graphql: ReturnType<typeof graphqlBoundary>

const screenLabel = () => i18n.t("materials.audioInstructions.screenLabel")
const languageLabel = () => i18n.t("materials.audioInstructions.languageLabel")

/** The material forms keep the assignment in their own state. */
function Assignment({kind, languages, value, onChange}: Scenario) {
    const [current, setCurrent] = useState(value)
    return (
        <AudioInstructionsFields
            kind={kind}
            languages={languages}
            value={current}
            onChange={(next) => {
                setCurrent(next)
                onChange(next)
            }}
        />
    )
}

const meta = {
    title: "Admin/Support materials/AudioInstructionsFields",
    component: AudioInstructionsFields,
    args: {
        kind: "audio/mpeg",
        languages: ["en", "es"],
        onChange: fn(),
    },
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <Assignment {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const NotInstructions: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("combobox", {name: screenLabel()})).toBeVisible()
        await expect(canvas.queryByRole("combobox", {name: languageLabel()})).toBeNull()
    },
}

export const AssignToAScreen: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: screenLabel()}))
        await userEvent.click(
            within(document.body).getByRole("option", {
                name: i18n.t("materials.audioInstructions.screens.ballot"),
            })
        )
        expect(args.onChange).toHaveBeenLastCalledWith({screen: "ballot", language: "en"})
        await userEvent.click(canvas.getByRole("combobox", {name: languageLabel()}))
        await userEvent.click(
            within(document.body).getByRole("option", {name: i18n.t("common.language.es")})
        )
        expect(args.onChange).toHaveBeenLastCalledWith({screen: "ballot", language: "es"})
    },
}

export const Assigned: Story = {
    args: {value: {screen: "review", language: "es"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("combobox", {name: screenLabel()})).toHaveTextContent(
            i18n.t("materials.audioInstructions.screens.review")
        )
        await expect(canvas.getByRole("combobox", {name: languageLabel()})).toHaveTextContent(
            i18n.t("common.language.es")
        )
    },
}

export const NotAudio: Story = {
    args: {kind: "application/pdf"},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).queryByRole("combobox")).toBeNull()
    },
}
