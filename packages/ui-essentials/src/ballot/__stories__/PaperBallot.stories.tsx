// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {Box} from "@mui/material"
import {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {
    ECandidatesLayout,
    ICandidate,
    IContest,
    IDecodedVoteChoice,
    IDecodedVoteContest,
} from "@sequentech/ui-core"

import {Question} from "../Question"
import {BallotEngine, BallotEngineProvider} from "../engine"
import {BallotSelectionPort, BallotSelectionProvider} from "../selection"
import {IBallotStyle} from "../types"
import {withCatalogue} from "./catalogue"

/**
 * Contests drawn with `presentation.candidates_layout = "paper-grid"`.
 *
 * Fictional names, in the shape of a large national ballot: a 66-candidate contest
 * choosing 12, and a 203-option contest choosing one. Resize to a phone to see the
 * grid fold to two columns.
 */

const SURNAMES = [
    "Alonzo",
    "Alvarez",
    "Andrada",
    "Angeles",
    "Aquino",
    "Aragon",
    "Bautista",
    "Belmonte",
    "Bernardo",
    "Blanco",
    "Cabrera",
    "Calderon",
    "Campos",
    "Canlas",
    "Castillo",
    "Castro",
    "Chua",
    "Cruz",
    "David",
    "De Guzman",
    "De Leon",
    "Del Rosario",
]
const GIVEN_NAMES = ["Amado", "Beatriz", "Carlo"]

const candidate = (contestId: string, id: string, name: string, order: number): ICandidate =>
    ({
        id,
        contest_id: contestId,
        name,
        description: "",
        presentation: {sort_order: order},
    }) as unknown as ICandidate

const senatorCandidates = (): Array<ICandidate> =>
    GIVEN_NAMES.flatMap((given, round) =>
        SURNAMES.map((surname, index) => {
            const order = round * SURNAMES.length + index
            return candidate("senator", `senator-${order}`, `${surname}, ${given} (IND)`, order)
        })
    )

const partyListCandidates = (): Array<ICandidate> =>
    Array.from({length: 203}, (_, index) =>
        candidate(
            "party-list",
            `party-${index}`,
            `Samahan ${String(index + 1).padStart(3, "0")}`,
            index
        )
    )

const paperContest = (
    id: string,
    name: string,
    maxVotes: number,
    candidates: Array<ICandidate>
): IContest =>
    ({
        id,
        election_id: "election",
        name,
        description: "",
        min_votes: 0,
        max_votes: maxVotes,
        winning_candidates_num: maxVotes,
        counting_algorithm: "plurality-at-large",
        candidates,
        presentation: {candidates_layout: ECandidatesLayout.PAPER_GRID},
    }) as unknown as IContest

const ballotStyleFor = (contest: IContest): IBallotStyle =>
    ({
        id: "style",
        election_id: "election",
        election_event_id: "event",
        tenant_id: "tenant",
        area_id: "area",
        ballot_eml: {
            id: "style",
            tenant_id: "tenant",
            election_event_id: "event",
            election_id: "election",
            area_id: "area",
            contests: [contest],
            election_event_presentation: {},
            election_presentation: {},
        },
    }) as unknown as IBallotStyle

const engine: BallotEngine = {
    sortCandidatesInContest: (candidates) =>
        [...candidates].sort(
            (a, b) => (a.presentation?.sort_order ?? 0) - (b.presentation?.sort_order ?? 0)
        ),
    isPreferential: () => false,
    checkIsBlank: (contest) => contest.choices.every((choice) => choice.selected < 0),
    getWriteInAvailableCharacters: () => 0,
    isEligibleAcclaimedCandidate: () => true,
}

const emptyMarks = (contest: IContest, selected: Array<string>): IDecodedVoteContest =>
    ({
        contest_id: contest.id,
        is_explicit_invalid: false,
        choices: contest.candidates.map((option) => ({
            id: option.id,
            selected: selected.includes(option.id) ? 0 : -1,
            write_in_text: "",
        })),
        invalid_errors: [],
        invalid_alerts: [],
    }) as unknown as IDecodedVoteContest

interface PaperBallotStoryProps {
    contest: IContest
    selected: Array<string>
    isReview: boolean
}

const PaperBallotStory: React.FC<PaperBallotStoryProps> = ({contest, selected, isReview}) => {
    const ballotStyle = useMemo(() => ballotStyleFor(contest), [contest])
    const [marks, setMarks] = useState<IDecodedVoteContest>(() => emptyMarks(contest, selected))
    const port: BallotSelectionPort = {
        contest: () => marks,
        choice: (_style, _contest, candidateId) =>
            marks.choices.find((choice) => choice.id === candidateId),
        setChoice: ({voteChoice}) =>
            setMarks((current) => ({
                ...current,
                choices: current.choices.map((choice: IDecodedVoteChoice) =>
                    choice.id === voteChoice.id ? {...choice, ...voteChoice} : choice
                ),
            })),
        setBlank: () => undefined,
        setInvalid: () => undefined,
        reset: () =>
            setMarks((current) => ({
                ...current,
                choices: current.choices.map((choice: IDecodedVoteChoice) => ({
                    ...choice,
                    selected: -1,
                })),
            })),
        isVoted: () => false,
        imageBaseUrl: "",
    }
    return (
        <BallotEngineProvider engine={engine}>
            <BallotSelectionProvider port={port}>
                <Box component="main" sx={{backgroundColor: "#ffffff", padding: "0 0 16px"}}>
                    <Question
                        ballotStyle={ballotStyle}
                        question={contest}
                        isReview={isReview}
                        errorSelectionState={[]}
                        setDecodedContests={() => undefined}
                    />
                </Box>
            </BallotSelectionProvider>
        </BallotEngineProvider>
    )
}

const meta: Meta<typeof PaperBallotStory> = {
    title: "ballot/PaperBallot",
    component: PaperBallotStory,
    parameters: {backgrounds: {default: "white"}},
    decorators: [withCatalogue],
}

export default meta

type Story = StoryObj<typeof PaperBallotStory>

const SENATOR_SELECTION = [
    "senator-0",
    "senator-4",
    "senator-9",
    "senator-17",
    "senator-22",
    "senator-30",
    "senator-38",
    "senator-41",
    "senator-50",
    "senator-57",
    "senator-61",
    "senator-65",
]

/** Sixty-six candidates, twelve to choose: four columns of seventeen. */
export const Senator66: Story = {
    args: {
        contest: paperContest("senator", "Senator", 12, senatorCandidates()),
        selected: SENATOR_SELECTION,
        isReview: false,
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByRole("checkbox")).toHaveLength(66)
        const unmarked = canvas.getByRole("checkbox", {name: /^2\. Alvarez, Amado/})
        await userEvent.click(unmarked)
        await expect(unmarked).toBeChecked()
        await userEvent.click(unmarked)
        await expect(unmarked).not.toBeChecked()
    },
}

/** Two hundred and three options, one to choose: the densest rows. */
export const PartyList203: Story = {
    args: {
        contest: paperContest("party-list", "Party List", 1, partyListCandidates()),
        selected: ["party-118"],
        isReview: false,
    },
}

/** The review screen keeps the paper look and the ballot numbers of what was marked. */
export const Senator66Review: Story = {
    args: {
        contest: paperContest("senator", "Senator", 12, senatorCandidates()),
        selected: SENATOR_SELECTION,
        isReview: true,
    },
}
