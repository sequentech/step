// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {UpsertArea} from "./UpsertArea"
import {
    AreaFormFixture,
    dataCalls,
    dataWrites,
    setUpAreaForm,
    upserts,
    type AreaFormServices,
} from "./__stories__/AreaFormFixture"

interface Scenario extends AreaFormServices {
    /** Whether the event's area list opens the form in a drawer, or a standalone route renders it. */
    drawer: boolean
    /** The area the drawer edits; without one it creates an area. */
    id?: string
    close: Mock<() => void>
}

const meta = {
    title: "Admin/Area/UpsertArea",
    component: UpsertArea,
    args: {drawer: true, upsert: "success", area: "record", close: fn()},
    argTypes: {
        id: {control: "select", options: [STORY_IDS.area]},
        upsert: {control: "inline-radio", options: ["success", "failure"]},
        area: {control: "inline-radio", options: ["record", "loading"]},
        drawer: {table: {disable: true}},
    },
    parameters: {
        // The drawers open on the event's page, whose route has an `id` of its own.
        router: {
            path: "/sequent_backend_election_event/:id",
            initialEntries: [`/sequent_backend_election_event/${EVENT_ID}`],
        },
    },
    beforeEach: ({args}) => setUpAreaForm(args),
    render: ({drawer, id, close}) => (
        <AreaFormFixture>
            {drawer ? (
                <UpsertArea
                    record={eventRecord() as Sequent_Backend_Election_Event}
                    id={id}
                    electionEventId={EVENT_ID}
                    close={close}
                />
            ) : (
                <UpsertArea />
            )}
        </AreaFormFixture>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const nameField = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: /Name/})

async function saved(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
    await waitFor(() => expect(upserts()).toHaveLength(1))
    return upserts()[0].variables
}

async function notified(message: string) {
    const notification = await within(document.body).findByText(message)
    await waitFor(() => expect(notification).toBeVisible())
}

const areaReads = () =>
    dataCalls().filter(
        ({method, args}) => method === "getOne" && args[0] === "sequent_backend_area"
    )

export const CreateInEventDrawer: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.type(await nameField(canvasElement), "East district")
        expect(await saved(canvasElement)).toEqual({
            name: "East district",
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            areaContestsIds: [],
            allow_early_voting: "no_early_voting",
        })
        await notified(i18n.t("areas.createAreaSuccess"))
        expect(args.close).toHaveBeenCalledTimes(1)
        // The event page's own `id` is not an area to edit, and the mutation alone creates it.
        expect(areaReads()).toEqual([])
        expect(dataWrites()).toEqual([])
    },
}

export const EditInEventDrawer: Story = {
    args: {id: STORY_IDS.area},
    parameters: {widgets: ["FormContentWrapper"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const name = await nameField(canvasElement)
        await waitFor(() => expect(name).toHaveValue("North district"))
        // The area's presentation allows early voting, but the event has no early voting channel.
        const earlyVoting = canvas.getByRole("switch", {
            name: i18n.t("areas.formImputs.allowEarlyVoting"),
        })
        await expect(earlyVoting).toBeChecked()
        await expect(earlyVoting).toBeDisabled()
        await userEvent.type(name, " and harbour")
        expect(await saved(canvasElement)).toMatchObject({
            id: STORY_IDS.area,
            name: "North district and harbour",
            electionEventId: EVENT_ID,
            areaContestsIds: [STORY_IDS.secondContest],
            allow_early_voting: "allow_early_voting",
        })
        await notified(i18n.t("areas.updateAreaSuccess"))
        expect(args.close).toHaveBeenCalledTimes(1)
        const read = {
            method: "getOne",
            args: ["sequent_backend_area", expect.objectContaining({id: STORY_IDS.area})],
        }
        // Saving refreshes the area; the mutation alone writes it.
        await waitFor(() => expect(areaReads()).toEqual([read, read]))
        expect(dataWrites()).toEqual([])
    },
}

export const LoadingArea: Story = {
    args: {id: STORY_IDS.area, area: "loading"},
    parameters: {widgets: ["FormContentWrapper"]},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(areaReads()).toHaveLength(1))
        // The form waits for the area instead of opening empty.
        expect(within(canvasElement).queryByRole("textbox", {name: /Name/})).toBeNull()
        expect(within(canvasElement).queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const EditRoute: Story = {
    args: {drawer: false},
    parameters: {
        widgets: ["FormContentWrapper"],
        router: {
            path: "/sequent_backend_area/:id",
            initialEntries: [`/sequent_backend_area/${STORY_IDS.area}`],
        },
    },
    play: async ({canvasElement}) => {
        const name = await nameField(canvasElement)
        await waitFor(() => expect(name).toHaveValue("North district"))
        await userEvent.type(name, " and harbour")
        // Without a drawer's event, the area's own event scopes the upsert.
        expect(await saved(canvasElement)).toMatchObject({
            id: STORY_IDS.area,
            name: "North district and harbour",
            electionEventId: EVENT_ID,
        })
        await notified(i18n.t("areas.updateAreaSuccess"))
        expect(within(canvasElement).queryByRole("combobox", {name: "Election Event"})).toBeNull()
    },
}

export const CreateRoute: Story = {
    args: {drawer: false},
    parameters: {
        router: {
            path: "/sequent_backend_area/create",
            initialEntries: [`/sequent_backend_area/create?electionEventId=${EVENT_ID}`],
        },
    },
    play: async ({canvasElement}) => {
        await userEvent.type(await nameField(canvasElement), "Harbour")
        // The event of the address scopes the new area, so the form offers no event choice.
        expect(within(canvasElement).queryByRole("combobox", {name: "Election Event"})).toBeNull()
        expect(await saved(canvasElement)).toEqual({
            name: "Harbour",
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            areaContestsIds: [],
            allow_early_voting: "no_early_voting",
        })
        await notified(i18n.t("areas.createAreaSuccess"))
        expect(areaReads()).toEqual([])
    },
}
