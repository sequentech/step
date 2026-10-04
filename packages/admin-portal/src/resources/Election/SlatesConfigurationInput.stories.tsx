// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {SimpleForm} from "react-admin"
import {EMobileCandidateLists, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SLATES_FORM_FIELD} from "@/utils/slates"
import {SlatesConfigurationInput} from "./SlatesConfigurationInput"

interface Scenario {
    /** The slate configuration the election already has, as text. */
    configuration: string
}

const CONFIGURATION = JSON.stringify(
    {
        version: 1,
        mobile_candidate_lists: EMobileCandidateLists.COLLAPSED,
        slates: [
            {
                id: "independent-voices",
                name: {en: "Independent Voices"},
                members: {"contest-1": ["candidate-1"]},
            },
        ],
    },
    null,
    2
)

let graphql: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Election/SlatesConfigurationInput",
    component: SlatesConfigurationInput,
    args: {configuration: CONFIGURATION},
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({configuration}) => (
        <AdminStoryProvider boundary={graphql}>
            <SimpleForm record={{id: 1, [SLATES_FORM_FIELD]: configuration}} toolbar={false}>
                <SlatesConfigurationInput
                    defaultLanguage="en"
                    contests={undefined}
                    candidates={undefined}
                    isCandidateListPartial={false}
                />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const label = (key: string) => i18n.t(`electionScreen.slates.${key}`)
const configurationInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: label("configuration")})
const mobileListsInput = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("combobox", {
        name: new RegExp(label("mobileCandidateLists.label")),
    })

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(configurationInput(canvasElement)).toHaveValue(CONFIGURATION)
        await expect(mobileListsInput(canvasElement)).toHaveTextContent(
            label("mobileCandidateLists.options.collapsed")
        )
    },
}

export const ChangesTheMobileCandidateLists: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(mobileListsInput(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: label("mobileCandidateLists.options.expanded"),
            })
        )
        await waitFor(() =>
            expect(mobileListsInput(canvasElement)).toHaveTextContent(
                label("mobileCandidateLists.options.expanded")
            )
        )
        expect(
            JSON.parse((configurationInput(canvasElement) as HTMLTextAreaElement).value)
                .mobile_candidate_lists
        ).toBe(EMobileCandidateLists.EXPANDED)
    },
}

export const WithoutSlates: Story = {
    args: {configuration: ""},
    play: async ({canvasElement}) => {
        await expect(configurationInput(canvasElement)).toHaveValue("")
        await expect(mobileListsInput(canvasElement)).toHaveAttribute("aria-disabled", "true")
    },
}
