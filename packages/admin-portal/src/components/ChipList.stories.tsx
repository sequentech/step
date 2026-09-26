// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {ListContextProvider, useList} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS, candidateRecords, storyId} from "@/__stories__/fixtures"
import {ChipList} from "./ChipList"

interface Scenario {
    /** Candidates of the contest row; none while the reference is loading. */
    candidates: number | "loading"
    /** The widget's `max`; ten when unset. */
    max?: number
    /** Called when a click reaches the list row around the chips. */
    onRowClick: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>

/** Alice and Bob, then further candidates up to `count`. */
const candidates = (count: number) =>
    Array.from({length: count}, (_, index) => {
        const [alice, bob] = candidateRecords()
        if (index === 0) return {...alice, name: "Alice Example"}
        if (index === 1) return {...bob, name: "Bob Example"}
        return {...alice, id: storyId(6, index + 1), name: `Candidate ${index + 1}`}
    })

function CandidateChips({candidates: count, max, onRowClick}: Scenario) {
    const list = useList(
        count === "loading" ? {isPending: true} : {data: candidates(count), perPage: 100}
    )
    return (
        <ListContextProvider value={list}>
            {/* A contest list row: clicking a chip must not also open the row. */}
            <table aria-label="Contests">
                <tbody>
                    <tr onClick={onRowClick}>
                        <td>
                            <ChipList
                                source="sequent_backend_candidate"
                                filterFields={["election_event_id", "contest_id"]}
                                max={max}
                            />
                        </td>
                    </tr>
                </tbody>
            </table>
        </ListContextProvider>
    )
}

const meta = {
    title: "Admin/Components/ChipList",
    component: ChipList,
    args: {candidates: 2, onRowClick: fn()},
    argTypes: {
        candidates: {control: "select", options: [0, 2, 12, "loading"]},
        max: {control: {type: "number", min: 1}},
    },
    beforeEach: async () => {
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql}>
            <CandidateChips {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const filter = JSON.stringify({election_event_id: EVENT_ID, contest_id: STORY_IDS.contest})
const links = (canvasElement: HTMLElement) =>
    within(within(canvasElement).getByRole("cell")).queryAllByRole("link")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const [alice, bob] = links(canvasElement)
        await expect(alice).toHaveTextContent("Alice Example")
        expect(alice).toHaveAttribute(
            "href",
            `/sequent_backend_candidate/${STORY_IDS.candidate}?filter=${filter}`
        )
        expect(bob).toHaveAttribute(
            "href",
            `/sequent_backend_candidate/${STORY_IDS.secondCandidate}?filter=${filter}`
        )
    },
}

export const OpensTheCandidateWithoutSelectingTheRow: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("link", {name: "Bob Example"}))
        await expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
            `/sequent_backend_candidate/${STORY_IDS.secondCandidate}?filter=${filter}`
        )
        expect(args.onRowClick).not.toHaveBeenCalled()
    },
}

export const ShowsAtMostTen: Story = {
    args: {candidates: 12},
    play: async ({canvasElement}) => {
        const shown = links(canvasElement)
        expect(shown).toHaveLength(10)
        await expect(shown[9]).toHaveTextContent("Candidate 10")
        expect(within(canvasElement).queryByText("Candidate 11")).not.toBeInTheDocument()
    },
}

export const CustomMaximum: Story = {
    args: {candidates: 12, max: 3},
    play: async ({canvasElement}) => {
        expect(links(canvasElement).map((link) => link.textContent)).toEqual([
            "Alice Example",
            "Bob Example",
            "Candidate 3",
        ])
    },
}

export const Empty: Story = {
    args: {candidates: 0},
    play: async ({canvasElement}) => {
        expect(links(canvasElement)).toEqual([])
    },
}

export const Loading: Story = {
    args: {candidates: "loading"},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).getByRole("cell")).toBeEmptyDOMElement()
    },
}
