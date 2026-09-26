// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {expect, within} from "storybook/test"
import {Meta, StoryObj} from "@storybook/react-vite"
import {INITIAL_VIEWPORTS} from "storybook/viewport"
import {Box} from "@mui/material"
import {
    IContest,
    ICandidate,
    ICountingAlgorithm,
    EInvalidVotePolicy,
    IDecodedVoteContest,
    initCore,
} from "@sequentech/ui-core"
import {PlaintextVoteContest} from "../PlaintextVoteContest"

const TENANT_ID = "tenant-1"
const ELECTION_EVENT_ID = "ee-2025"
const ELECTION_ID = "election-1"
const CONTEST_ID = "contest-mayor"

const baseCandidates: ICandidate[] = [
    {
        id: "1",
        tenant_id: TENANT_ID,
        election_event_id: ELECTION_EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        name: "Alice Johnson",
        description: "Running for a better tomorrow",
    },
    {
        id: "2",
        tenant_id: TENANT_ID,
        election_event_id: ELECTION_EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        name: "Bob Smith",
        description: "Committed to public service and community values",
    },
    {
        id: "3",
        tenant_id: TENANT_ID,
        election_event_id: ELECTION_EVENT_ID,
        election_id: ELECTION_ID,
        contest_id: CONTEST_ID,
        name: "Carol White",
        description: "Championing sustainability and innovation",
    },
]

const writeInCandidate: ICandidate = {
    id: "write-in-1",
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    contest_id: CONTEST_ID,
    name: "",
    presentation: {is_write_in: true},
}

const invalidVoteCandidate: ICandidate = {
    id: "invalid-1",
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    contest_id: CONTEST_ID,
    name: "Mark as Invalid Vote",
    presentation: {is_explicit_invalid: true},
}

const explicitBlankCandidate: ICandidate = {
    id: "blank-1",
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    contest_id: CONTEST_ID,
    name: "Cast Explicit Blank Vote",
    presentation: {is_explicit_blank: true},
}

const makeContest = (overrides: Partial<IContest> = {}): IContest => ({
    id: CONTEST_ID,
    tenant_id: TENANT_ID,
    election_event_id: ELECTION_EVENT_ID,
    election_id: ELECTION_ID,
    name: "Mayor of Springfield",
    max_votes: 3,
    min_votes: 0,
    winning_candidates_num: 1,
    is_encrypted: true,
    candidates: baseCandidates,
    ...overrides,
})

const makePlaintext = (overrides: Partial<IDecodedVoteContest> = {}): IDecodedVoteContest => ({
    contest_id: CONTEST_ID,
    is_explicit_invalid: false,
    is_decline_to_vote: false,
    is_blank_ballot: false,
    invalid_errors: [],
    invalid_alerts: [],
    choices: [
        {id: "1", selected: 0},
        {id: "2", selected: -1},
        {id: "3", selected: -1},
    ],
    ...overrides,
})

const pointsLabel = (points: number) => `(${points} pts)`

const ContestWrapper: React.FC<{children: React.ReactNode}> = ({children}) => (
    <Box sx={{maxWidth: 600, padding: 2}}>{children}</Box>
)

const meta: Meta<typeof PlaintextVoteContest> = {
    title: "components/PlaintextVoteContest",
    component: PlaintextVoteContest,
    parameters: {
        backgrounds: {
            default: "white",
        },
        viewport: {
            viewports: INITIAL_VIEWPORTS,
            defaultViewport: "iphone6",
        },
    },
    loaders: [
        async () => {
            await initCore()
            return {}
        },
    ],
    decorators: [
        (Story) => (
            <ContestWrapper>
                <Story />
            </ContestWrapper>
        ),
    ],
}

export default meta

type Story = StoryObj<typeof PlaintextVoteContest>

const commonParameters = {
    viewport: {
        disable: true,
    },
}

export const NormalVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Alice Johnson")).toBeVisible()
        await expect(canvas.getByText("Bob Smith")).toBeVisible()
        await expect(canvas.queryByText("Carol White")).not.toBeInTheDocument()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest()}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: 0},
                    {id: "2", selected: 0},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const SingleSelection: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Alice Johnson")).toBeVisible()
        await expect(canvas.queryByText("Bob Smith")).not.toBeInTheDocument()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest({max_votes: 1})}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: 0},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const BlankVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Blank Vote")).toBeVisible()
        await expect(canvas.queryByText("Alice Johnson")).not.toBeInTheDocument()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest()}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: -1},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const ExplicitInvalidVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Mark as Invalid Vote")).toBeVisible()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest({
                candidates: [...baseCandidates, invalidVoteCandidate],
                presentation: {invalid_vote_policy: EInvalidVotePolicy.ALLOWED},
            })}
            questionPlaintext={makePlaintext({
                is_explicit_invalid: true,
                choices: [
                    {id: "1", selected: -1},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                    {id: "invalid-1", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["list", "listitem"],
        },
    },
}

export const ExplicitBlankVote: Story = {
    render: () => (
        <PlaintextVoteContest
            question={makeContest({
                candidates: [...baseCandidates, explicitBlankCandidate],
            })}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: -1},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                    {id: "blank-1", selected: 0},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: commonParameters,
}

export const WithValidationWarnings: Story = {
    render: () => (
        <PlaintextVoteContest
            question={makeContest()}
            questionPlaintext={makePlaintext({
                invalid_errors: [
                    {
                        error_type: "Implicit",
                        message: "Too many candidates selected for this contest",
                        message_map: {},
                    },
                    {
                        error_type: "Implicit",
                        message: "Minimum number of selections not met",
                        message_map: {},
                    },
                ] as any,
                choices: [
                    {id: "1", selected: 0},
                    {id: "2", selected: 0},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const PreferentialVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("1st")).toBeVisible()
        await expect(canvas.getByText("2nd")).toBeVisible()
        await expect(canvas.getByText("3rd")).toBeVisible()
        const rows = canvas.getAllByRole("listitem")
        await expect(rows.map((row) => row.textContent)).toEqual([
            expect.stringContaining("Carol White"),
            expect.stringContaining("Alice Johnson"),
            expect.stringContaining("Bob Smith"),
        ])
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest({
                counting_algorithm: ICountingAlgorithm.INSTANT_RUNOFF,
                max_votes: 3,
            })}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "3", selected: 0},
                    {id: "1", selected: 1},
                    {id: "2", selected: 2},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const WriteInVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("textbox")).toHaveValue("David Chen")
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest({
                candidates: [...baseCandidates, writeInCandidate],
                presentation: {allow_writeins: true},
            })}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: 0},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                    {id: "write-in-1", selected: 0, write_in_text: "David Chen"},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const WithPoints: Story = {
    render: () => (
        <PlaintextVoteContest
            question={makeContest({
                presentation: {
                    show_points: true,
                    cumulative_number_of_checkboxes: 3,
                },
            })}
            questionPlaintext={makePlaintext({
                choices: [
                    {id: "1", selected: 2},
                    {id: "2", selected: 1},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const ContestNotFound: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Contest not found (ID: contest-mayor)")).toBeVisible()
    },
    render: () => (
        <PlaintextVoteContest
            question={null}
            questionPlaintext={makePlaintext()}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found (ID: contest-mayor)"
            markedInvalidLabel="Marked as invalid"
            pointsLabel={pointsLabel}
            isDeclineToVotePolicyEnabled={false}
        />
    ),
    parameters: commonParameters,
}

export const DeclineToVote: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Declined to vote")).toBeVisible()
        await expect(canvas.queryByText("Ballot explicitly marked invalid")).not.toBeInTheDocument()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest()}
            questionPlaintext={makePlaintext({
                is_decline_to_vote: true,
                choices: [
                    {id: "1", selected: -1},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Ballot explicitly marked invalid"
            pointsLabel={() => ""}
            isDeclineToVotePolicyEnabled={true}
            declineToVoteLabel="Declined to vote"
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}

export const BlankBallot: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Blank ballot")).toBeVisible()
        await expect(canvas.queryByText("Alice Johnson")).not.toBeInTheDocument()
    },
    render: () => (
        <PlaintextVoteContest
            question={makeContest()}
            questionPlaintext={makePlaintext({
                is_blank_ballot: true,
                choices: [
                    {id: "1", selected: -1},
                    {id: "2", selected: -1},
                    {id: "3", selected: -1},
                ],
            })}
            publicBucketUrl=""
            contestNotFoundLabel="Contest not found"
            markedInvalidLabel="Ballot explicitly marked invalid"
            pointsLabel={() => ""}
            isDeclineToVotePolicyEnabled={false}
            isBlankBallotsPolicyEnabled={true}
            blankBallotLabel="Blank ballot"
        />
    ),
    parameters: {
        ...commonParameters,
        expectedFailure: {
            reason: "Rendered ballot choices are not direct children of their semantic list.",
            a11y: ["listitem"],
        },
    },
}
