// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {Route, Routes} from "react-router-dom"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import EditElectionEventTextData from "./EditElectionEventTextData"
import {localizedEvent} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

interface Scenario {
    /** What reading the election event does. */
    reads: ReadState
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

// The event tabs render the section on the event's route.
function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            tenant={tenant}
        >
            <ResourceContextProvider value={RESOURCE}>
                <Routes>
                    <Route path={`/${RESOURCE}/:id`} element={<EditElectionEventTextData />} />
                    <Route path={`/${RESOURCE}`} element={null} />
                </Routes>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventTextData",
    component: EditElectionEventTextData,
    args: {reads: "records"},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        router: {initialEntries: [`/${RESOURCE}/${EVENT_ID}`]},
        expectedFailure: {
            reason: "The row's edit and delete actions are icon buttons without accessible names.",
            a11y: ["button-name"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary({[RESOURCE]: [localizedEvent()]}, {reads: args.reads})
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const localization = (key: string) => i18n.t(`electionEventScreen.localization.${key}`)
const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Current location")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("row", {name: /welcome.*Welcome, voters/})
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0], args[1]])).toEqual([
            ["getOne", RESOURCE, expect.objectContaining({id: EVENT_ID})],
        ])
    },
}

export const AddAnOverride: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("row", {name: /welcome/})
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.add")}))
        const body = within(document.body)
        const title = await body.findByText(localization("common.title"))
        await waitFor(() => expect(title).toBeVisible())
        const form = within(title.closest("form") as HTMLElement)
        await userEvent.type(
            form.getByRole("textbox", {name: localization("labels.key")}),
            "greeting"
        )
        await userEvent.type(
            form.getByRole("textbox", {name: localization("labels.value")}),
            "Good morning"
        )
        await userEvent.click(form.getByRole("button", {name: "Save"}))
        // The table writes the event directly; the edit view shows the updated record.
        await expect(await canvas.findByRole("row", {name: /greeting.*Good morning/})).toBeVisible()
        expect(data.writes.map(({method, resource}) => [method, resource])).toEqual([
            ["update", RESOURCE],
        ])
        await waitFor(() => expect(body.queryByRole("dialog")).toBeNull())
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        // Until the event arrives the table has no overrides to list.
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("combobox", {name: localization("selectLanguage")})
        ).toBeVisible()
        expect(canvas.queryByRole("row", {name: /welcome/})).toBeNull()
        expect(data.calls.map(({method}) => method)).toEqual(["getOne"])
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const notice = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(notice).toBeVisible())
        await waitFor(() => expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`))
    },
}
