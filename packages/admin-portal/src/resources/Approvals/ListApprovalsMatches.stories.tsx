// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {Sequent_Backend_Applications} from "@/gql/graphql"
import {ApplicationsError, IApplicationsStatus} from "@/types/applications"
import {ListApprovalsMatches} from "./ListApprovalsMatches"
import {applicationRecord} from "./__stories__/ApprovalsFixture"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    graphqlCalls,
    listFilters,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    status: IApplicationsStatus
    /** What the status service answers to the approval. */
    answer: "approved" | "already-approved"
    goBack: Mock<() => void>
}

const meta = {
    title: "Admin/Approvals/ListApprovalsMatches",
    component: ListApprovalsMatches,
    args: {
        reads: "records",
        empty: false,
        status: IApplicationsStatus.PENDING,
        answer: "approved",
        goBack: fn(),
    },
    argTypes: {status: {control: "select", options: Object.values(IApplicationsStatus)}},
    parameters: {
        expectedFailure: {
            reason: "The approve action is an icon button without a name.",
            a11y: ["button-name"],
        },
    },
    beforeEach: ({args}) =>
        setUpApprovals(args, {
            ChangeApplicationStatus: () => ({
                data: {
                    ApplicationChangeStatus: {
                        message: "Application approved",
                        error:
                            args.answer === "already-approved"
                                ? ApplicationsError.APPROVED_VOTER
                                : null,
                    },
                },
            }),
        }),
    render: ({status, goBack}) => (
        <ApprovalsScreen>
            <ListApprovalsMatches
                electionEventId={EVENT_ID}
                task={applicationRecord({status}) as Sequent_Backend_Applications}
                goBack={goBack}
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const voterRow = (canvasElement: HTMLElement, username: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(username)})

async function approve(canvasElement: HTMLElement) {
    const alice = await voterRow(canvasElement, "alice@example.test")
    await userEvent.click(within(alice).getByRole("button"))
    const dialogElement = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialogElement).toBeVisible())
    await userEvent.click(within(dialogElement).getByRole("button", {name: "Approve"}))
    await waitFor(() => expect(dialogElement).not.toBeInTheDocument())
}

const approvals = () => graphqlCalls().filter(({name}) => name === "ChangeApplicationStatus")

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(await voterRow(canvasElement, "alice@example.test")).toBeVisible()
        // The search attributes of the application preload the name filters.
        await waitFor(() =>
            expect(listFilters("user").at(-1)).toMatchObject({
                tenant_id: TENANT_ID,
                election_event_id: EVENT_ID,
                first_name: {IsLike: "Alice"},
                last_name: {IsLike: "Example"},
            })
        )
        expect(approvals()).toEqual([])
    },
}

export const ApproveTheVoter: Story = {
    play: async ({canvasElement, args}) => {
        await approve(canvasElement)
        await waitFor(() => expect(approvals()).toHaveLength(1))
        expect(approvals()[0].variables).toEqual({
            tenant_id: TENANT_ID,
            id: APPLICATION_ID,
            user_id: STORY_IDS.user,
            area_id: STORY_IDS.area,
            election_event_id: EVENT_ID,
        })
        const message = await within(document.body).findByText("Voter approved")
        await waitFor(() => expect(message).toBeVisible())
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const VoterAlreadyApproved: Story = {
    args: {answer: "already-approved"},
    play: async ({canvasElement, args}) => {
        await approve(canvasElement)
        const message = await within(document.body).findByText("Voter is already approved.")
        await waitFor(() => expect(message).toBeVisible())
        expect(approvals()).toHaveLength(1)
        expect(args.goBack).not.toHaveBeenCalled()
    },
}

export const AcceptedApplication: Story = {
    args: {status: IApplicationsStatus.ACCEPTED},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const alice = await voterRow(canvasElement, "alice@example.test")
        // An accepted application offers no voter to approve.
        expect(within(alice).queryByRole("button")).toBeNull()
        expect(approvals()).toEqual([])
    },
}
