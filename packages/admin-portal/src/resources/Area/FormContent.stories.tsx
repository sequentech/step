// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import {FormContent} from "./FormContent"
import {
    AreaFormFixture,
    choose,
    contestReads,
    northDistrict,
    secondEvent,
    setUpAreaForm,
    upserts,
    type AreaFormServices,
} from "./__stories__/AreaFormFixture"

interface Scenario extends AreaFormServices {
    /** Creating an area, or editing North district. */
    mode: "create" | "edit"
    /** Whether the form belongs to an event, which then fixes the area's event. */
    fromEvent: boolean
    /** Whether the event has the early voting channel. */
    earlyVotingChannel: boolean
    weightedVoting: boolean
    close: Mock<() => void>
}

const meta = {
    title: "Admin/Area/FormContent",
    component: FormContent,
    args: {
        mode: "create",
        fromEvent: true,
        earlyVotingChannel: false,
        weightedVoting: false,
        upsert: "success",
        area: "record",
        close: fn(),
    },
    argTypes: {
        mode: {control: "inline-radio", options: ["create", "edit"]},
        upsert: {control: "inline-radio", options: ["success", "failure"]},
        area: {table: {disable: true}},
    },
    beforeEach: ({args}) => setUpAreaForm(args),
    render: ({mode, fromEvent, earlyVotingChannel, weightedVoting, close}) => {
        const event = eventRecord(undefined, {
            voting_channels: {online: true, early_voting: earlyVotingChannel},
        })
        const form = (
            <FormContent
                record={fromEvent ? (event as never) : undefined}
                electionEventId={fromEvent ? EVENT_ID : undefined}
                id={mode === "edit" ? STORY_IDS.area : undefined}
                area_presentation={mode === "edit" ? northDistrict.presentation : undefined}
                weightedVotingForAreas={weightedVoting}
                close={close}
            />
        )
        return (
            <AreaFormFixture>
                {mode === "edit" ? (
                    <RecordContextProvider value={northDistrict}>{form}</RecordContextProvider>
                ) : (
                    form
                )}
            </AreaFormFixture>
        )
    },
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const field = (canvasElement: HTMLElement, name: string | RegExp) =>
    within(canvasElement).getByRole("textbox", {name})
const combobox = (canvasElement: HTMLElement, name: string | RegExp) =>
    within(canvasElement).getByRole("combobox", {name})
const save = (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
/** Waits for the notification's snackbar to finish entering. */
async function notified(message: string) {
    const notification = await within(document.body).findByText(message)
    await waitFor(() => expect(notification).toBeVisible())
}

/** The variables of the one upsert the form sent. */
async function upserted() {
    await waitFor(() => expect(upserts()).toHaveLength(1))
    return upserts()[0].variables
}

export const CreateArea: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("areas.common.title"))).toBeVisible()
        await userEvent.type(field(canvasElement, /Name/), "East district")
        await userEvent.type(field(canvasElement, /Description/), "East district voters")
        // React-admin labels the contest input from its source: the reference input's label is unused.
        await choose(combobox(canvasElement, "Area contest"), "Members")
        await choose(combobox(canvasElement, "Parent"), "North district")
        // The event has no early voting channel.
        await expect(
            canvas.getByRole("switch", {name: i18n.t("areas.formImputs.allowEarlyVoting")})
        ).toBeDisabled()
        await save(canvasElement)
        expect(await upserted()).toEqual({
            name: "East district",
            description: "East district voters",
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            parentId: STORY_IDS.area,
            areaContestsIds: [STORY_IDS.contest],
            allow_early_voting: "no_early_voting",
        })
        await notified(i18n.t("areas.createAreaSuccess"))
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const EditArea: Story = {
    args: {mode: "edit"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(field(canvasElement, /Name/)).toHaveValue("North district"))
        // The area's contests come from its area-contest rows.
        await expect(await canvas.findByRole("button", {name: "Deputies"})).toBeVisible()
        const description = field(canvasElement, /Description/)
        await userEvent.clear(description)
        await userEvent.type(description, "North district, including the harbour")
        await save(canvasElement)
        expect(await upserted()).toEqual({
            id: STORY_IDS.area,
            name: "North district",
            description: "North district, including the harbour",
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
            parentId: null,
            areaContestsIds: [STORY_IDS.secondContest],
            annotations: {},
            labels: {},
            type: "district",
            allow_early_voting: "allow_early_voting",
        })
        await notified(i18n.t("areas.updateAreaSuccess"))
        // Saving refetches the area's contests.
        await waitFor(() => expect(contestReads()).toHaveLength(2))
        expect(contestReads()[1].variables).toEqual({
            electionEventId: EVENT_ID,
            areaId: STORY_IDS.area,
        })
    },
}

export const SaveFailureNotifies: Story = {
    args: {upsert: "failure"},
    play: async ({canvasElement, args}) => {
        await userEvent.type(field(canvasElement, /Name/), "East district")
        await save(canvasElement)
        await waitFor(() => expect(upserts()).toHaveLength(1))
        await notified(i18n.t("areas.createAreaError"))
        expect(args.close).toHaveBeenCalledTimes(1)
    },
}

export const NameIsRequired: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.type(field(canvasElement, /Description/), "Unnamed voters")
        await save(canvasElement)
        await expect(await within(canvasElement).findByText("Required")).toBeVisible()
        expect(upserts()).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const EarlyVotingChannel: Story = {
    args: {earlyVotingChannel: true},
    play: async ({canvasElement}) => {
        await userEvent.type(field(canvasElement, /Name/), "East district")
        const earlyVoting = within(canvasElement).getByRole("switch", {
            name: i18n.t("areas.formImputs.allowEarlyVoting"),
        })
        await expect(earlyVoting).toBeEnabled()
        await userEvent.click(earlyVoting)
        await save(canvasElement)
        expect(await upserted()).toMatchObject({allow_early_voting: "allow_early_voting"})
    },
}

export const WeightedAreas: Story = {
    args: {weightedVoting: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.type(field(canvasElement, /Name/), "East district")
        const weight = canvas.getByRole("spinbutton", {name: "Weight"})
        await expect(weight).toHaveValue(1)
        await userEvent.clear(weight)
        await userEvent.type(weight, "3")
        await save(canvasElement)
        expect(await upserted()).toMatchObject({annotations: {weight: 3}})
    },
}

export const ChoosesTheEvent: Story = {
    args: {fromEvent: false},
    play: async ({canvasElement}) => {
        // The event select replaces its disabled placeholder once the events have loaded.
        await choose(
            await within(canvasElement).findByRole("combobox", {name: "Election Event"}),
            "Referendum"
        )
        await userEvent.type(field(canvasElement, /Name/), "Harbour")
        await save(canvasElement)
        expect(await upserted()).toMatchObject({
            name: "Harbour",
            electionEventId: secondEvent.id,
        })
    },
}
