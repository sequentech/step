// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {SimpleForm} from "react-admin"
import {EMobileCandidateLists, i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {candidateRecords, contestRecord} from "@/__stories__/fixtures"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {Sequent_Backend_Contest} from "@/gql/graphql"
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
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Election/SlatesConfigurationInput",
    component: SlatesConfigurationInput,
    args: {configuration: CONFIGURATION},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
        data = resourceBoundary(
            args.configuration ? {sequent_backend_candidate: candidateRecords()} : {}
        )
    },
    render: ({configuration}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SimpleForm record={{id: 1, [SLATES_FORM_FIELD]: configuration}} toolbar={false}>
                <SlatesConfigurationInput
                    defaultLanguage="en"
                    contests={[contestRecord()] as Sequent_Backend_Contest[]}
                    tenantId={TENANT_ID}
                    electionEventId={EVENT_ID}
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
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_candidate")
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
        expect(data.calls).toEqual([])
    },
}
