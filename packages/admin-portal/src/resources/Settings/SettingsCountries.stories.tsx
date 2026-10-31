// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsCountries} from "./SettingsCountries"
import {TENANT_RESOURCE, settingsTab, settingsTenant} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the tenant does. */
    reads: ReadState
    /** Whether the access service rejects the country lists. */
    failure: boolean
}

const Tab = settingsTab(SettingsCountries)

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsCountries",
    component: SettingsCountries,
    args: {reads: "records", failure: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary({[TENANT_RESOURCE]: [settingsTenant()]}, {reads: args.reads})
        graphql = graphqlBoundary(
            {
                limitAccessByCountries: () =>
                    args.failure
                        ? {errors: [new GraphQLError("Synthetic access service failure")]}
                        : {data: {limit_access_by_countries: {success: true}}},
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: () => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceContextProvider value={TENANT_RESOURCE}>
                <Tab />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

/** The voting and the enrollment country lists, in that order. */
const countryInputs = async (canvasElement: HTMLElement) => {
    await waitFor(() => expect(within(canvasElement).getByText("Spain")).toBeVisible())
    return within(canvasElement).getAllByRole("combobox", {name: "Countries"})
}

async function blockEnrollmentFromFrance(canvasElement: HTMLElement) {
    const [, enrollment] = await countryInputs(canvasElement)
    await userEvent.type(enrollment, "Fran")
    await userEvent.click(await within(document.body).findByRole("option", {name: "France"}))
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await countryInputs(canvasElement)
        await expect(canvas.getByText(i18n.t("settings.countries.title"))).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(graphql.calls).toEqual([])
    },
}

export const BlockEnrollmentFromACountry: Story = {
    play: async ({canvasElement}) => {
        await blockEnrollmentFromFrance(canvasElement)
        await waitFor(() =>
            expect(graphql.calls).toEqual([
                {
                    name: "limitAccessByCountries",
                    variables: {votingCountries: ["ES"], enrollCountries: ["FR"]},
                    headers: {},
                },
            ])
        )
        await within(document.body).findByText("Element updated")
        expect(data.writes).toEqual([])
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(data.writes).toEqual([
                {
                    method: "update",
                    resource: TENANT_RESOURCE,
                    params: expect.objectContaining({
                        id: TENANT_ID,
                        data: {
                            settings: expect.objectContaining({
                                voting_countries: ["ES"],
                                enroll_countries: ["FR"],
                                gapi_email: "calendar@admin-story.invalid",
                            }),
                        },
                    }),
                },
            ])
        )
    },
}

export const AccessServiceFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement}) => {
        await blockEnrollmentFromFrance(canvasElement)
        const message = await within(document.body).findByText(
            i18n.t("settings.countries.error.errorSaving")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(graphql.calls.map(({name}) => name)).toEqual(["limitAccessByCountries"])
        expect(data.writes).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByText("Spain")).toBeNull()
    },
}
