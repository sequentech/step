// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {eventRecord} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ViewApproval} from "./ViewApproval"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    SECOND_APPLICATION_ID,
    reads,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    /** The application the administrator opened. */
    applicationId: string
    isModal: boolean
    goBack: Mock<() => void>
}

const meta = {
    title: "Admin/Approvals/ViewApproval",
    component: ViewApproval,
    args: {
        reads: "records",
        empty: false,
        applicationId: APPLICATION_ID,
        isModal: false,
        goBack: fn(),
    },
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "The approve action is an icon button without a name.",
            a11y: ["button-name"],
        },
    },
    beforeEach: ({args}) => setUpApprovals(args),
    render: ({applicationId, isModal, goBack}) => (
        <ApprovalsScreen>
            <ViewApproval
                electionEventId={EVENT_ID}
                currApprovalId={applicationId}
                goBack={goBack}
                electionEventRecord={eventRecord() as Sequent_Backend_Election_Event}
                isModal={isModal}
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const details = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("table", {name: "approvals details table"})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const table = within(await details(canvasElement))
        await expect(await table.findByRole("row", {name: /Alice/})).toBeVisible()
        await expect(table.getByRole("row", {name: /alice@example.test/})).toBeVisible()
        await expect(table.getByRole("row", {name: /Madrid/})).toBeVisible()
        await expect(
            within(canvasElement).getByRole("button", {name: "Reject Application"})
        ).toBeVisible()
        expect(reads("getOne", "sequent_backend_applications")[0].args[1]).toMatchObject({
            id: APPLICATION_ID,
        })
    },
}

export const AcceptedApplication: Story = {
    args: {applicationId: SECOND_APPLICATION_ID},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const table = within(await details(canvasElement))
        await expect(await table.findByRole("row", {name: /Bob/})).toBeVisible()
        expect(within(canvasElement).queryByRole("button", {name: "Reject Application"})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the application arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_applications")).toHaveLength(1))
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
    },
}

export const AsDialog: Story = {
    args: {isModal: true},
    play: async ({args}) => {
        const dialogElement = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialogElement).toBeVisible())
        const dialog = within(dialogElement)
        const table = within(await dialog.findByRole("table", {name: "approvals details table"}))
        await expect(table.getByRole("row", {name: /alice@example.test/})).toBeVisible()
        await expect(dialog.getByRole("heading", {name: "Task Information"})).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        expect(args.goBack).toHaveBeenCalled()
    },
}

export const GoBack: Story = {
    play: async ({canvasElement, args}) => {
        await details(canvasElement)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Back"}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const OpenTheRejection: Story = {
    // The open dialog hides the matches list, and its unnamed buttons, from axe.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await details(canvasElement)
        await userEvent.click(
            within(canvasElement).getByRole("button", {name: "Reject Application"})
        )
        const dialogElement = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialogElement).toBeVisible())
        await expect(
            within(dialogElement).getByRole("combobox", {name: /Rejection Reason/})
        ).toBeVisible()
    },
}
