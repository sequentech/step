// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useRef, useState} from "react"
import type {Meta, StoryObj} from "@storybook/react"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {Button} from "@mui/material"
import type {RaRecord} from "react-admin"
import {initCore} from "@sequentech/ui-core"
import type {
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Sheet,
} from "@/gql/graphql"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {dataBoundary} from "@/__stories__/dataBoundary"
import {EditTallySheet} from "./EditTallySheet"

const ELECTION_ID = "33333333-3333-4333-8333-333333333333"
const CONTEST_ID = "44444444-4444-4444-8444-444444444444"
const AREA_ID = "55555555-5555-4555-8555-555555555555"
const SHEET_ID = "66666666-6666-4666-8666-666666666666"
const ALICE_ID = "77777777-7777-4777-8777-777777777771"
const BOB_ID = "77777777-7777-4777-8777-777777777772"
const scope = {tenant_id: TENANT_ID, election_event_id: EVENT_ID, election_id: ELECTION_ID}
const election: Sequent_Backend_Election = {
    id: ELECTION_ID,
    ...scope,
    contests: [],
    contests_aggregate: {nodes: []},
}
const onSubmit = fn()
let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof dataBoundary>
let contest: Sequent_Backend_Contest
let sheet: Sequent_Backend_Tally_Sheet

interface Scenario {
    draft: boolean
    omitBlank: boolean
    siblingBlank: number | null
    maxVotes: number
    counting: string
}

function Fixture({draft}: Scenario) {
    const submitRef = useRef<HTMLButtonElement>(null)
    const [disabled, setDisabled] = useState(true)
    const [chosen, setChosen] = useState<Sequent_Backend_Contest | undefined>(contest)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <main>
                <EditTallySheet
                    election={election}
                    choosenContest={chosen}
                    setChoosenContest={setChosen}
                    tallySheet={draft ? undefined : sheet}
                    submitRef={submitRef}
                    setIsButtonDisabled={setDisabled}
                    doCreatedTalySheet={onSubmit}
                />
                <Button disabled={disabled} onClick={() => submitRef.current?.click()}>
                    Save tally sheet
                </Button>
            </main>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally sheet",
    component: Fixture,
    args: {
        draft: false,
        omitBlank: false,
        siblingBlank: null,
        maxVotes: 1,
        counting: "plurality-at-large",
    },
    beforeEach: async ({args}) => {
        await initCore()
        localStorage.removeItem("tallySheetData")
        onSubmit.mockClear()
        boundary = graphqlBoundary({})
        contest = {
            ...scope,
            id: CONTEST_ID,
            presentation: {i18n: {en: {name: "Council representative"}}},
            max_votes: args.maxVotes,
            counting_algorithm: args.counting,
        } as Sequent_Backend_Contest
        sheet = {
            ...scope,
            id: SHEET_ID,
            area_id: AREA_ID,
            contest_id: CONTEST_ID,
            channel: "PAPER",
            content: {
                area_id: AREA_ID,
                contest_id: CONTEST_ID,
                total_votes: 10,
                total_valid_votes: 8,
                invalid_votes: {implicit_invalid: 1, explicit_invalid: 1, total_invalid: 2},
                total_blank_votes: 0,
                blank_ballots: args.omitBlank ? undefined : 0,
                census: 20,
                candidate_results: {
                    [ALICE_ID]: {candidate_id: ALICE_ID, total_votes: 5},
                    [BOB_ID]: {candidate_id: BOB_ID, total_votes: 3},
                },
            },
        } as Sequent_Backend_Tally_Sheet
        if (args.draft) localStorage.setItem("tallySheetData", JSON.stringify(sheet))
        const candidates = [
            {
                id: ALICE_ID,
                ...scope,
                contest_id: CONTEST_ID,
                presentation: {i18n: {en: {name: "Alice Example"}}},
            },
            {
                id: BOB_ID,
                ...scope,
                contest_id: CONTEST_ID,
                presentation: {i18n: {en: {name: "Bob Example"}}},
            },
        ]
        const areas = [{id: AREA_ID, ...scope, name: "North precinct", parent_id: null}]
        const siblings =
            args.siblingBlank === null
                ? []
                : [
                      {
                          ...sheet,
                          id: "sibling-sheet",
                          contest_id: "other-contest",
                          version: 2,
                          content: {
                              ...sheet.content,
                              contest_id: "other-contest",
                              blank_ballots: args.siblingBlank,
                          },
                      },
                  ]
        data = dataBoundary({
            getList: async <RecordType extends RaRecord>(resource: string) => {
                const records: Record<string, RaRecord[]> = {
                    sequent_backend_contest: [contest],
                    sequent_backend_candidate: candidates,
                    sequent_backend_area: areas,
                    sequent_backend_area_contest: [
                        {id: "area-contest", area_id: AREA_ID, contest_id: CONTEST_ID},
                    ],
                    sequent_backend_tally_sheet: siblings,
                }
                if (!(resource in records)) {
                    data.unexpected.push(resource)
                    throw new Error(`Unexpected list ${resource}`)
                }
                return {data: records[resource] as RecordType[], total: records[resource].length}
            },
        })
        return () => {
            expect(boundary.unexpected).toEqual([])
            expect(data.unexpected).toEqual([])
            boundary.client.stop()
            localStorage.removeItem("tallySheetData")
        }
    },
} satisfies Meta<typeof Fixture>
export default meta
type Story = StoryObj<typeof meta>

async function ready(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await waitFor(() =>
        expect(canvas.getByRole("textbox", {name: "Total Valid Votes"})).toHaveValue("8")
    )
    await waitFor(() =>
        expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
    )
    await waitFor(() =>
        expect(canvas.getByRole("combobox", {name: "Search Area"})).toHaveValue("North precinct")
    )
    return canvas
}

async function replace(input: HTMLElement, value: string) {
    await userEvent.clear(input)
    await userEvent.type(input, value)
}

export const ValidSheetSavesScopedNumericResults: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0]).toEqual({
            ...scope,
            id: SHEET_ID,
            area_id: AREA_ID,
            contest_id: CONTEST_ID,
            channel: "PAPER",
            content: {
                area_id: AREA_ID,
                contest_id: CONTEST_ID,
                total_votes: 10,
                total_valid_votes: 8,
                invalid_votes: {implicit_invalid: 1, explicit_invalid: 1, total_invalid: 2},
                total_blank_votes: 0,
                blank_ballots: 0,
                census: 20,
                candidate_results: {
                    [ALICE_ID]: {candidate_id: ALICE_ID, total_votes: 5},
                    [BOB_ID]: {candidate_id: BOB_ID, total_votes: 3},
                },
            },
        })
        expect(JSON.parse(localStorage.getItem("tallySheetData")!)).toEqual(
            onSubmit.mock.calls[0][0]
        )
        expect(boundary.calls).toEqual([])
        const siblingQueries = data.calls.filter(
            ({args}) => args[0] === "sequent_backend_tally_sheet"
        )
        expect(siblingQueries.length).toBeGreaterThan(0)
        expect(siblingQueries[0].args[1]).toMatchObject({
            filter: {
                ...scope,
                area_id: AREA_ID,
                channel: {format: "hasura-raw-query", value: {_eq: "PAPER"}},
                status: {format: "hasura-raw-query", value: {_eq: "APPROVED"}},
            },
        })
    },
}

export const CandidateArithmeticBlocksAndRecovers: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await replace(canvas.getByRole("textbox", {name: "Total Valid Votes"}), "7")
        await expect(
            canvas.getByText(/Candidate votes \(8\) must be between 7 and 7/)
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeDisabled()
        expect(onSubmit).not.toHaveBeenCalled()
        await replace(canvas.getByRole("textbox", {name: "Total Valid Votes"}), "8")
        await expect(canvas.queryByText(/Candidate votes/)).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
    },
}

export const CensusBoundBlocksAndRecovers: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await replace(canvas.getByRole("textbox", {name: "Census"}), "9")
        await expect(
            canvas.getByText("Total votes (10) must not be greater than census (9)")
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeDisabled()
        await replace(canvas.getByRole("textbox", {name: "Census"}), "10")
        await expect(canvas.queryByText(/must not be greater than census/)).not.toBeInTheDocument()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
    },
}

export const InvalidVotesRecalculateTotals: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await replace(canvas.getByRole("textbox", {name: "Explicitly Invalid Votes"}), "3")
        await expect(canvas.getByRole("textbox", {name: "Total Invalid Votes"})).toHaveValue("4")
        await expect(canvas.getAllByRole("textbox", {name: "Total Votes"})[0]).toHaveValue("12")
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0].content).toMatchObject({
            total_votes: 12,
            invalid_votes: {implicit_invalid: 1, explicit_invalid: 3, total_invalid: 4},
        })
    },
}

export const DraftRestoresAndCreatesWithoutExistingId: Story = {
    args: {draft: true},
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0]).toMatchObject({
            ...scope,
            channel: "PAPER",
            content: {total_votes: 10, census: 20},
        })
        expect(onSubmit.mock.calls[0][0]).not.toHaveProperty("id")
    },
}

export const BlankBallotsPrefillFromZeroBlankContest: Story = {
    args: {omitBlank: true},
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await expect(canvas.getByRole("textbox", {name: "Blank Ballots"})).toHaveValue("0")
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0].content.blank_ballots).toBe(0)
    },
}

export const SiblingBlankDisagreementWarnsWithoutPreventingCorrection: Story = {
    args: {siblingBlank: 1},
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await expect(
            canvas.getByText(
                "Blank Ballots must have the same value on every contest sheet of this ballot box"
            )
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0].content.blank_ballots).toBe(0)
    },
}

export const ExistingBlankBallotEntryIsNotOverwritten: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await replace(canvas.getByRole("textbox", {name: "Blank Ballots"}), "1")
        await expect(
            canvas.getByText(
                "Blank Ballots value is outside the range implied by this box's per-contest blank vote counts"
            )
        ).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Blank Ballots"})).toHaveValue("1")
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
    },
}

export const MultiMarkContestUsesConfiguredBounds: Story = {
    args: {maxVotes: 2},
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await replace(canvas.getByRole("textbox", {name: "Total Valid Votes"}), "4")
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
        await replace(canvas.getByRole("textbox", {name: "Total Valid Votes"}), "3")
        await expect(
            canvas.getByText(/Candidate votes \(8\) must be between 3 and 6/)
        ).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeDisabled()
        expect(onSubmit).not.toHaveBeenCalled()
        await replace(canvas.getByRole("textbox", {name: "Total Valid Votes"}), "4")
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
    },
}

export const PostalChannelIsSavedAndScopesSiblingSheets: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        await userEvent.click(canvas.getByRole("combobox", {name: "Channel"}))
        await userEvent.click(
            within(canvasElement.ownerDocument.body).getByRole("option", {name: "POSTAL"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0].channel).toBe("POSTAL")
        await waitFor(() =>
            expect(
                data.calls.filter(({args}) => args[0] === "sequent_backend_tally_sheet").at(-1)
                    ?.args[1]
            ).toMatchObject({
                filter: {channel: {format: "hasura-raw-query", value: {_eq: "POSTAL"}}},
            })
        )
    },
}

export const MissingCandidateCountBlocksAndRecovers: Story = {
    play: async ({canvasElement}) => {
        const canvas = await ready(canvasElement)
        const aliceRow = within(canvas.getByText("Alice Example").parentElement!)
        const count = aliceRow.getByRole("textbox", {name: "Total Votes"})
        await userEvent.clear(count)
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeDisabled()
        await expect(
            canvas.getByText(/Candidate votes \(3\) must be between 8 and 8/)
        ).toBeVisible()
        expect(onSubmit).not.toHaveBeenCalled()
        await userEvent.type(count, "5")
        await expect(canvas.getByRole("button", {name: "Save tally sheet"})).toBeEnabled()
        await userEvent.click(canvas.getByRole("button", {name: "Save tally sheet"}))
        await waitFor(() => expect(onSubmit).toHaveBeenCalledTimes(1))
        expect(onSubmit.mock.calls[0][0].content.candidate_results).toEqual({
            [ALICE_ID]: {candidate_id: ALICE_ID, total_votes: 5},
            [BOB_ID]: {candidate_id: BOB_ID, total_votes: 3},
        })
    },
}
