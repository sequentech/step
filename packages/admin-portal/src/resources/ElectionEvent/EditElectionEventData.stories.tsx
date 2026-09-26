// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {Route, Routes} from "react-router-dom"
import {i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {EditElectionEventData} from "./EditElectionEventData"
import {
    SECOND_ELECTION_ID,
    eventDataArgs,
    eventDataBoundaries,
    type EventDataScenario,
} from "./__stories__/EventDataFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election_event"

let boundaries: ReturnType<typeof eventDataBoundaries>

// The event tabs render the section on the event's route.
function Fixture() {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={boundaries.graphql}
            dataProvider={boundaries.data.provider}
            role={permissions}
            tenant={tenant}
        >
            <WidgetsContextProvider>
                <ResourceContextProvider value={RESOURCE}>
                    <Routes>
                        <Route path={`/${RESOURCE}/:id`} element={<EditElectionEventData />} />
                        <Route path={`/${RESOURCE}`} element={null} />
                    </Routes>
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventData",
    component: EditElectionEventData,
    args: eventDataArgs,
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {router: {initialEntries: [`/${RESOURCE}/${EVENT_ID}`]}},
    beforeEach: async ({args}) => {
        boundaries = eventDataBoundaries(args)
        await Promise.all([boundaries.graphql.ready, initCore()])
    },
    render: (_args, {globals}) => <Fixture key={JSON.stringify(globals)} />,
} satisfies WidgetMeta<EventDataScenario>
export default meta
type Story = StoryObj<EventDataScenario>

const field = (key: string) => i18n.t(`electionEventScreen.field.${key}`)
const location = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Current location")

async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByRole("textbox", {name: field("name")})).toHaveValue(
        "Council event"
    )
    return canvas
}

const writes = (resource: string) =>
    boundaries.data.writes.filter((write) => write.resource === resource)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await loaded(canvasElement)
        expect(
            boundaries.data.calls.find(
                ({method, args: [resource]}) => method === "getOne" && resource === RESOURCE
            )?.args[1]
        ).toMatchObject({id: EVENT_ID})
        expect(boundaries.data.writes).toEqual([])
    },
}

export const SaveTheEvent: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const description = canvas.getByRole("textbox", {name: field("description")})
        await userEvent.clear(description)
        await userEvent.type(description, "Council members for 2026")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(writes(RESOURCE)).toHaveLength(1))
        const [write] = writes(RESOURCE)
        expect(write.method).toBe("update")
        // The event's name, alias and description follow its English presentation.
        expect(write.params.data).toMatchObject({
            id: EVENT_ID,
            name: "Council event",
            alias: "Council",
            description: "Council members for 2026",
            presentation: {
                language_conf: {enabled_language_codes: ["en", "es"], default_language_code: "en"},
            },
        })
        for (const key of ["enabled_languages", "electionsOrder", "resultsWebsitePolicy"])
            expect(write.params.data).not.toHaveProperty(key)
        expect(writes("sequent_backend_election")).toEqual([])
        // The saved event is read again.
        await waitFor(() =>
            expect(
                boundaries.data.calls.filter(
                    ({method, args: [resource]}) => method === "getOne" && resource === RESOURCE
                ).length
            ).toBeGreaterThan(1)
        )
    },
}

export const SaveACustomElectionOrder: Story = {
    args: {customOrder: true},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const description = canvas.getByRole("textbox", {name: field("description")})
        await userEvent.type(description, ".")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(writes(RESOURCE)).toHaveLength(1))
        // The elections, sorted by their order, are numbered from zero.
        expect(
            writes("sequent_backend_election").map(({params}) => [
                params.id,
                (params.data as {presentation: {sort_order: number}}).presentation.sort_order,
            ])
        ).toEqual([
            [SECOND_ELECTION_ID, 0],
            [STORY_IDS.election, 1],
        ])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(boundaries.data.calls.map(({method}) => method)).toContain("getOne")
        )
        expect(within(canvasElement).queryByRole("textbox", {name: field("name")})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const notice = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(notice).toBeVisible())
        await waitFor(() => expect(location(canvasElement).textContent).toBe(`/${RESOURCE}`))
    },
}
