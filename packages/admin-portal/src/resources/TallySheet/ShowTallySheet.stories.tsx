// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {Button} from "@mui/material"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {Sequent_Backend_Contest, Sequent_Backend_Tally_Sheet_Insert_Input} from "@/gql/graphql"
import {
    AREAS,
    CANDIDATES,
    CONTEST,
    TALLY_SHEETS,
    sheetContent,
} from "./__stories__/TallySheetFixture"
import {ShowTallySheet} from "./ShowTallySheet"

interface Scenario {
    /** Whether the sheet is a stored version or one the wizard has just entered. */
    stored: boolean
    candidateReads: ReadState
    /** A save fails with this message. */
    writeError?: string
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const SHEET = TALLY_SHEETS[2]
/** What the wizard's edit step hands over: the sheet without an id or version. */
const ENTERED_SHEET: Sequent_Backend_Tally_Sheet_Insert_Input = {
    tenant_id: SHEET.tenant_id,
    election_event_id: SHEET.election_event_id,
    election_id: SHEET.election_id,
    contest_id: SHEET.contest_id,
    area_id: SHEET.area_id,
    channel: SHEET.channel,
    content: SHEET.content,
}

function Fixture({stored}: Scenario) {
    const submitRef = useRef<HTMLButtonElement>(null)
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ShowTallySheet
                tallySheet={stored ? SHEET : ENTERED_SHEET}
                contest={CONTEST as Sequent_Backend_Contest}
                submitRef={submitRef}
            />
            {/* The wizard submits the sheet through this reference. */}
            <Button onClick={() => submitRef.current?.click()}>Submit tally sheet</Button>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally sheet/ShowTallySheet",
    component: ShowTallySheet,
    args: {stored: true, candidateReads: "records"},
    argTypes: {candidateReads: {control: "inline-radio", options: ["records", "loading"]}},
    parameters: {
        expectedFailure: {
            reason:
                "The area and channel selects show an InputLabel that is not linked to them, " +
                "so they have no accessible name.",
            a11y: ["aria-input-field-name"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {
                sequent_backend_candidate: CANDIDATES,
                sequent_backend_tally_sheet: TALLY_SHEETS,
            },
            {reads: {sequent_backend_candidate: args.candidateReads}, writeError: args.writeError}
        )
        graphql = graphqlBoundary(
            {
                sequent_backend_contest_extended: () => ({
                    data: {
                        sequent_backend_area_contest: AREAS.map(({id, name}) => ({
                            area: {id, name},
                        })),
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const field = (canvasElement: HTMLElement, name: string) =>
    canvasElement.querySelector<HTMLInputElement>(`input[name="${name}"]`)

async function loaded(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await canvas.findByText("Alice")
    await waitFor(() => expect(canvas.getByText("North district")).toBeVisible())
    return canvas
}

const expectedSheet = {
    tenant_id: STORY_IDS.tenant,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    contest_id: STORY_IDS.contest,
    area_id: STORY_IDS.area,
    channel: "PAPER",
    content: sheetContent(STORY_IDS.area, [32, 20]),
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await expect(canvas.getByText("PAPER")).toBeVisible()
        const values = Object.fromEntries(
            [
                "total_votes",
                "total_valid_votes",
                "total_invalid",
                "implicit_invalid",
                "explicit_invalid",
                "total_blank_votes",
                "blank_ballots",
                "census",
            ].map((name) => [name, field(canvasElement, name)?.value])
        )
        expect(values).toEqual({
            total_votes: "60",
            total_valid_votes: "55",
            total_invalid: "5",
            implicit_invalid: "2",
            explicit_invalid: "3",
            total_blank_votes: "5",
            blank_ballots: "3",
            census: "80",
        })
        // Read only: every count is disabled.
        for (const input of Array.from(canvasElement.querySelectorAll("input[type=number]"))) {
            expect(input).toBeDisabled()
        }
        const candidates = [STORY_IDS.candidate, STORY_IDS.secondCandidate].map((id) => [
            canvasElement.querySelector(`[id="${id}"]`)?.closest("div.MuiBox-root")
                ?.firstElementChild?.textContent,
            canvasElement.querySelector<HTMLInputElement>(`input[id="${id}"]`)?.value,
        ])
        expect(candidates).toEqual([
            ["Alice", "32"],
            ["Bob", "20"],
        ])
        expect(graphql.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "sequent_backend_contest_extended",
                variables: {
                    electionEventId: EVENT_ID,
                    contestId: STORY_IDS.contest,
                    tenantId: STORY_IDS.tenant,
                },
            },
        ])
        expect(data.writes).toEqual([])
    },
}

export const SaveAStoredVersion: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Submit tally sheet"}))
        await expect(await within(document.body).findByText("Tally Sheet saved")).toBeVisible()
        expect(data.writes).toEqual([
            {
                method: "update",
                resource: "sequent_backend_tally_sheet",
                params: expect.objectContaining({id: SHEET.id, data: expectedSheet}),
            },
        ])
    },
}

export const SaveAnEnteredSheet: Story = {
    args: {stored: false},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Submit tally sheet"}))
        await expect(await within(document.body).findByText("Tally Sheet saved")).toBeVisible()
        expect(data.writes).toEqual([
            {
                method: "create",
                resource: "sequent_backend_tally_sheet",
                params: expect.objectContaining({data: expectedSheet}),
            },
        ])
    },
}

export const SaveFailure: Story = {
    args: {writeError: "Synthetic save rejected"},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Submit tally sheet"}))
        await expect(
            await within(document.body).findByText("Error saving Tally Sheet")
        ).toBeVisible()
        expect(data.writes).toHaveLength(1)
    },
}

export const LoadingCandidates: Story = {
    args: {candidateReads: "loading"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_candidate")
        )
        await expect(canvas.getByText("Candidates")).toBeVisible()
        // The counts are filled in once the candidates have loaded.
        expect(field(canvasElement, "total_votes")?.value).toBe("")
        expect(canvas.queryByText("Alice")).toBeNull()
    },
}
