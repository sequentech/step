// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SigningRequestPanel} from "@/components/signing/SigningRequestPanel"
import {
    ANA,
    CODE,
    JOSE,
    MARIA,
    MEMBERS,
    ORGANIZATIONS,
    POST,
    fakeApi,
    makePanel,
    memoryStorage,
    signedInAs,
} from "@/components/signing/__stories__/fixtures"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {ClosedVotingCard} from "./ClosedVotingCard"

interface Scenario {
    /** How many of the Post's members must sign the closing. */
    required: number
    status: SigningRequestStatus
    /** What the seal record shows of each country seal the close made. */
    seals: Array<Record<string, unknown>>
    /** A result stored before `seals` existed: `seal: null` and no `seals`. */
    legacy?: boolean
    /** The election event's number format, as the panel names it. */
    numberFormatPolicy?: string
}

const SPAIN_SHA512 = "3f9a".repeat(32)
const PORTUGAL_SHA512 = "c0d7".repeat(32)

const CLOSED_AT = "2028-05-12T11:01:00Z"

let boundary: ReturnType<typeof graphqlBoundary>
let api: ReturnType<typeof fakeApi>
let signers: typeof MEMBERS

/** A close voting request signed by its first `required` members, as the server keeps its result. */
async function closing({
    required,
    status,
    seals,
    legacy,
    numberFormatPolicy,
}: Scenario): Promise<ISigningPanelData> {
    signers = MEMBERS.slice(0, required)
    const panel = await makePanel({
        action: SigningAction.CloseVoting,
        status,
        required,
        signed: signers,
        subject: {channels: ["ONLINE"]},
    })
    if (status !== SigningRequestStatus.Executed) return panel
    return {
        ...panel,
        number_format_policy: numberFormatPolicy ?? null,
        request: {
            ...panel.request,
            execution_result: {
                closed_at: CLOSED_AT,
                channels: ["ONLINE"],
                code: CODE,
                signatures: signers.map((person) => ({
                    user_id: person.userId,
                    username: person.username,
                    display_name: person.name,
                    signed_at: CLOSED_AT,
                })),
                ...(legacy ? {seal: null} : {seals}),
            },
        },
    }
}

const meta = {
    title: "Admin/Publish/ClosedVotingCard",
    component: ClosedVotingCard,
    args: {required: 2, status: SigningRequestStatus.Executed, seals: []},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({})
        api = fakeApi(await closing(args))
    },
    render: () => (
        <AdminStoryProvider
            boundary={boundary}
            roles={[IPermissions.SIGN_CLOSE_VOTING]}
            auth={signedInAs(JOSE)}
            tenantRecord={ORGANIZATIONS.first}
        >
            <SigningRequestPanel
                requestId="55555555-5555-4555-8555-555555555555"
                api={api as ISigningApi}
                open
                onClose={fn()}
                completionActions={(data) => <ClosedVotingCard data={data} />}
                storage={memoryStorage()}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const card = async () => {
    const element = await within(document.body).findByTestId("closed-voting")
    await waitFor(() => expect(element).toBeVisible())
    return within(element)
}

const rowValue = (view: ReturnType<typeof within>, label: string) =>
    view.getByRole("rowheader", {name: label}).nextElementSibling?.textContent

export const ClosedWithTheClosingSignatures: Story = {
    parameters: {widgets: ["RowsTable"]},
    play: async ({args}) => {
        const view = await card()
        await expect(view.getByRole("heading", {name: /^Voting closed at .*\.$/})).toBeVisible()
        expect(view.queryByText(/Ballots sealed/)).toBeNull()
        expect(rowValue(view, "Closing signatures in the seal record")).toBe(
            `${args.required}, signing code ${CODE}`
        )
        expect(rowValue(view, "Signed by the members")).toBe(
            signers.map((person) => person.name).join(", ")
        )
        // Without a seal, no seal fields.
        expect(view.queryByRole("rowheader", {name: "Ballots in the seal"})).toBeNull()
        expect(view.queryByRole("rowheader", {name: /^Seal /})).toBeNull()
        await expect(
            within(document.body).getByRole("heading", {name: `Closing · ${POST} · Spain`})
        ).toBeVisible()
        // The action's description claims no seal: nothing seals until VOTE-FREEZE.
        await expect(
            within(document.body).getByText(
                "Started in Publish with Stop voting. Closes voting at the Post; the closing signatures are kept in its record."
            )
        ).toBeVisible()
        expect(within(document.body).queryByText(/seals the ballots/)).toBeNull()
    },
}

/** A Post where all three members sign the closing. */
export const ThreeMembersClose: Story = {
    args: {required: 3},
    play: async ({args}) => {
        const view = await card()
        expect(rowValue(view, "Closing signatures in the seal record")).toBe(
            `${args.required}, signing code ${CODE}`
        )
        expect(rowValue(view, "Signed by the members")).toBe(
            [MARIA, JOSE, ANA].map((person) => person.name).join(", ")
        )
    },
}

/** VOTE-FREEZE sealed the ballot box of each country under the Post. */
export const SealedByVoteFreeze: Story = {
    args: {
        seals: [
            {
                area_id: "11111111-1111-4111-8111-111111111111",
                area_name: "Spain",
                hash_algorithm: "SHA-512",
                hash: SPAIN_SHA512,
                ballots: 1356,
                signed_by: "The seal key, then published to the bulletin board",
            },
            {
                area_id: "22222222-2222-4222-8222-222222222222",
                area_name: null,
                hash_algorithm: "SHA-512",
                hash: PORTUGAL_SHA512,
                ballots: null,
                signed_by: null,
            },
        ],
    },
    play: async ({args}) => {
        const view = await card()
        await expect(view.getByRole("heading", {name: /Ballots sealed\.$/})).toBeVisible()
        // One block per country, named by the country, else its id.
        const spain = within(view.getByRole("table", {name: "Spain"}))
        // In the event's number format: the default, for an event without one.
        expect(rowValue(spain, "Ballots in the seal")).toBe("1,356")
        expect(rowValue(spain, "Seal SHA-512")).toBe("3f9a3f9a…3f9a3f9a")
        expect(rowValue(spain, "Signed by")).toBe(args.seals[0]?.signed_by)
        const second = within(
            view.getByRole("table", {name: "22222222-2222-4222-8222-222222222222"})
        )
        expect(rowValue(second, "Seal SHA-512")).toBe("c0d7c0d7…c0d7c0d7")
        expect(second.queryByRole("rowheader", {name: "Ballots in the seal"})).toBeNull()
        expect(second.queryByRole("rowheader", {name: "Signed by"})).toBeNull()
        // The closing rows follow the seals.
        expect(rowValue(view, "Closing signatures in the seal record")).toBe(
            `${args.required}, signing code ${CODE}`
        )
        // The algorithm comes from the seals, never a fixed one.
        expect(view.queryByText(/SHA-256/)).toBeNull()
    },
}

/** An event whose number format is 1.234.567,89 counts its seals' ballots in it. */
export const SealedInTheEventsNumberFormat: Story = {
    args: {
        numberFormatPolicy: "period-comma",
        seals: [
            {
                area_id: "11111111-1111-4111-8111-111111111111",
                area_name: "Spain",
                hash_algorithm: "SHA-512",
                hash: SPAIN_SHA512,
                ballots: 1234567,
                signed_by: null,
            },
        ],
    },
    play: async () => {
        const view = await card()
        const spain = within(view.getByRole("table", {name: "Spain"}))
        expect(rowValue(spain, "Ballots in the seal")).toBe("1.234.567")
    },
}

/** A result stored before seals were summaries: `seal: null` and no `seals`. */
export const ClosedBeforeSealSummaries: Story = {
    args: {legacy: true},
    play: async ({args}) => {
        const view = await card()
        await expect(view.getByRole("heading", {name: /^Voting closed at .*\.$/})).toBeVisible()
        expect(view.queryByText(/Ballots sealed/)).toBeNull()
        expect(view.queryByRole("rowheader", {name: /^Seal /})).toBeNull()
        expect(rowValue(view, "Closing signatures in the seal record")).toBe(
            `${args.required}, signing code ${CODE}`
        )
    },
}

export const WaitingForTheClosingToRun: Story = {
    args: {status: SigningRequestStatus.Completed},
    play: async () => {
        const pending = await within(document.body).findByTestId("closed-voting-pending")
        await waitFor(() => expect(pending).toBeVisible())
        await expect(pending).toHaveTextContent("Voting closes in a moment.")
        expect(within(document.body).queryByTestId("closed-voting")).toBeNull()
    },
}
