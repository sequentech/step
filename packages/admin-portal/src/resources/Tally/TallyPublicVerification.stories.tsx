// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import type {PublicVerificationReport} from "@/services/BrowserTrustee"
import type {TallyVerificationBoard, TallyVerifier} from "@/services/TallyPublicVerification"
import {TallyPublicVerification} from "./TallyPublicVerification"

const SESSION = "5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d"
const KEY = "tally.publicVerification"

const named = (name: string) => ({i18n: {en: {name}}})

const board = (
    batch: number,
    owner: Pick<
        TallyVerificationBoard,
        "election_presentation" | "contest_presentation" | "area_name"
    >
): TallyVerificationBoard => ({
    dkg_board: "dkg_0123456789abcdef0123456789abcdef",
    tally_board: `tally_fedcba9876543210fedcba9876543210_${batch}`,
    batch,
    ...owner,
    configuration_message: "Y29uZmlndXJhdGlvbg==",
    ballots_message: btoa(`ballots ${batch}`),
})

const COUNCIL = {
    election_presentation: named("Council"),
    contest_presentation: named("Members"),
}
const NORTH = board(0, {...COUNCIL, area_name: "North"})
const SOUTH = board(32, {...COUNCIL, area_name: "South"})
// A batch no contest area of the session owns.
const UNNAMED = board(64, {
    election_presentation: null,
    contest_presentation: null,
    area_name: null,
})

/**
 * A board's outcome: its report's status, why its transcript could not be
 * read, or a transcript that never arrives.
 */
type Outcome = {status: PublicVerificationReport["status"]} | {error: string} | {pending: true}

interface Scenario {
    finished: boolean
    boards: TallyVerificationBoard[]
    /** Outcomes by tally board; verified unless given. */
    outcomes: Record<string, Outcome>
    /** The boards cannot be read from the platform. */
    unreadable: boolean
}

const DETAIL: Record<PublicVerificationReport["status"], string> = {
    verified:
        "DKG, ballot proofs, ordered shuffles, endorsements, threshold decryption and plaintexts verified",
    incomplete: "Tally transcript is incomplete",
    invalid: "invalid shuffle proof",
}

/** The WASM verifier's contract, answering each board's outcome. */
function storyVerifier(outcomes: Record<string, Outcome>): TallyVerifier {
    const outcome = (tally: string): Outcome => outcomes[tally] ?? {status: "verified"}
    return {
        configurationHash: (message) => `configuration(${message})`,
        ballotsHash: (message) => `ballots(${message})`,
        tallyIdentifier: () => "42",
        fetchTranscript: async (_url, dkg, tally = "") => {
            const result = outcome(tally)
            if ("error" in result) throw result.error
            if ("pending" in result) return new Promise<string>(() => undefined)
            return JSON.stringify({version: 1, dkg_messages: [dkg], tally_messages: [tally]})
        },
        verifyTranscript: (transcript, anchors) => {
            const tally: string = JSON.parse(transcript).tally_messages[0]
            const {status} = outcome(tally) as {status: PublicVerificationReport["status"]}
            return JSON.stringify({
                status,
                detail: DETAIL[status],
                configuration_hash: JSON.parse(anchors).configuration_hash,
                tally_id: "42",
                public_key_hash: null,
                ballot_count: status === "verified" ? 120 : null,
                plaintexts: [],
                ballot_inclusions: [],
            } satisfies PublicVerificationReport)
        },
    }
}

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]

function Fixture({finished, outcomes}: Scenario) {
    return (
        <AdminStoryProvider boundary={boundary}>
            <TallyPublicVerification
                electionEventId={EVENT_ID}
                tallySessionId={SESSION}
                finished={finished}
                verifier={async () => storyVerifier(outcomes)}
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyPublicVerification",
    component: TallyPublicVerification,
    args: {finished: true, boards: [NORTH, SOUTH], outcomes: {}, unreadable: false},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                GetTallyPublicVerificationBoards: () =>
                    args.unreadable
                        ? {
                              errors: [
                                  new GraphQLError("the tally session's boards could not be read"),
                              ],
                          }
                        : {
                              data: {
                                  get_tally_public_verification_boards: {boards: args.boards},
                              },
                          },
            },
            {schema: true}
        )
        await boundary.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const verify = async (canvasElement: HTMLElement) =>
    userEvent.click(within(canvasElement).getByRole("button", {name: i18n.t(`${KEY}.verify`)}))

const rows = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .getAllByRole("row")
        .slice(1)
        .map((row) =>
            within(row)
                .getAllByRole("cell")
                .map((cell) => cell.textContent)
        )

export const AllBatchesVerified: Story = {
    parameters: {widgets: ["VerificationResults"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await verify(canvasElement)
        await expect(
            await canvas.findByText(i18n.t(`${KEY}.summary.verified`, {count: 2, total: 2}))
        ).toBeVisible()
        for (const check of ["ballots", "mixing", "decryption"])
            await expect(canvas.getByText(i18n.t(`${KEY}.checks.${check}`))).toBeVisible()
        expect(rows(canvasElement)).toEqual([
            [
                `Council · Members · North${NORTH.tally_board}`,
                "120",
                i18n.t(`${KEY}.status.verified`),
                DETAIL.verified,
            ],
            [
                `Council · Members · South${SOUTH.tally_board}`,
                "120",
                i18n.t(`${KEY}.status.verified`),
                DETAIL.verified,
            ],
        ])
        // The anchors' records are read with the tally results permission.
        expect(boundary.calls).toEqual([
            {
                name: "GetTallyPublicVerificationBoards",
                variables: {electionEventId: EVENT_ID, tallySessionId: SESSION},
                headers: {"x-hasura-role": "tally-results-read"},
            },
        ])
        expect(boundary.unexpected).toEqual([])
    },
}

export const FailedProofAndUnreadableBatch: Story = {
    parameters: {widgets: ["VerificationResults"]},
    args: {
        boards: [NORTH, SOUTH, UNNAMED],
        outcomes: {
            [NORTH.tally_board]: {status: "invalid"},
            [SOUTH.tally_board]: {error: "The board service did not answer"},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await verify(canvasElement)
        await expect(
            await canvas.findByText(i18n.t(`${KEY}.summary.invalid`, {count: 1, total: 3}))
        ).toBeVisible()
        expect(canvas.queryByText(i18n.t(`${KEY}.checks.title`))).toBeNull()
        expect(rows(canvasElement)).toEqual([
            [
                `Council · Members · North${NORTH.tally_board}`,
                "",
                i18n.t(`${KEY}.status.invalid`),
                DETAIL.invalid,
            ],
            [
                `Council · Members · South${SOUTH.tally_board}`,
                "",
                i18n.t(`${KEY}.status.error`),
                "The board service did not answer",
            ],
            [
                `${i18n.t(`${KEY}.batch`, {batch: 64})}${UNNAMED.tally_board}`,
                "120",
                i18n.t(`${KEY}.status.verified`),
                DETAIL.verified,
            ],
        ])
    },
}

export const IncompleteBatch: Story = {
    parameters: {widgets: ["VerificationResults"]},
    args: {outcomes: {[SOUTH.tally_board]: {status: "incomplete"}}},
    play: async ({canvasElement}) => {
        await verify(canvasElement)
        await expect(
            await within(canvasElement).findByText(
                i18n.t(`${KEY}.summary.incomplete`, {count: 1, total: 2})
            )
        ).toBeVisible()
    },
}

export const DownloadEvidence: Story = {
    parameters: {widgets: ["VerificationResults"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await verify(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: i18n.t(`${KEY}.download`)}))
        await waitFor(() =>
            expect(downloads.map(({name}) => name)).toEqual([
                `tally-${SESSION}-public-verification.json`,
            ])
        )
        await expect(canvas.getByRole("button", {name: i18n.t(`${KEY}.verifyAgain`)})).toBeEnabled()
    },
}

export const VerificationInProgress: Story = {
    args: {outcomes: {[SOUTH.tally_board]: {pending: true}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await verify(canvasElement)
        const progress = i18n.t(`${KEY}.progress`, {current: 2, total: 2})
        await expect(await canvas.findByRole("progressbar", {name: progress})).toBeVisible()
        await expect(canvas.getByText(progress)).toBeVisible()
        await expect(canvas.getByRole("button", {name: i18n.t(`${KEY}.verify`)})).toBeDisabled()
    },
}

export const TallyWithoutBoards: Story = {
    args: {boards: []},
    play: async ({canvasElement}) => {
        await verify(canvasElement)
        await expect(await within(canvasElement).findByText(i18n.t(`${KEY}.empty`))).toBeVisible()
    },
}

export const BoardsCannotBeRead: Story = {
    args: {unreadable: true},
    play: async ({canvasElement}) => {
        await verify(canvasElement)
        await expect(
            await within(canvasElement).findByText(
                i18n.t(`${KEY}.failed`, {
                    message: "the tally session's boards could not be read",
                })
            )
        ).toBeVisible()
    },
}

export const TallyNotFinished: Story = {
    args: {finished: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("button", {name: i18n.t(`${KEY}.verify`)})).toBeDisabled()
        await expect(canvas.getByText(i18n.t(`${KEY}.unavailable`))).toBeVisible()
    },
}
