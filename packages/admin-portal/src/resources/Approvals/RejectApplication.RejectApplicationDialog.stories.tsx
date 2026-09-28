// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Applications} from "@/gql/graphql"
import {IApplicationsStatus} from "@/types/applications"
import {RejectApplicationDialog} from "./RejectApplication"
import {APPLICATION_ID, applicationRecord} from "./__stories__/ApprovalsFixture"

interface Scenario {
    /** Whether the status service rejects the change. */
    failure: boolean
    status: IApplicationsStatus
    rejectDialogOpen: boolean
    goBack: () => void
    setRejectDialogOpen: (open: boolean) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Approvals/RejectApplicationDialog",
    component: RejectApplicationDialog,
    args: {
        failure: false,
        status: IApplicationsStatus.PENDING,
        rejectDialogOpen: true,
        goBack: fn(),
        setRejectDialogOpen: fn(),
    },
    argTypes: {status: {control: "select", options: Object.values(IApplicationsStatus)}},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                ChangeApplicationStatus: () =>
                    args.failure
                        ? {errors: [new GraphQLError("Synthetic status service failure")]}
                        : {
                              data: {
                                  ApplicationChangeStatus: {
                                      message: "Application rejected",
                                      error: null,
                                  },
                              },
                          },
            },
            {schema: true}
        )
        await boundary.ready
    },
    render: ({status, failure: _failure, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <RejectApplicationDialog
                {...args}
                electionEventId={STORY_IDS.event}
                task={applicationRecord({status}) as Sequent_Backend_Applications}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const reason = (value: string) => i18n.t(`approvalsScreen.reject.reasons.${value}`)

async function openDialog() {
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return within(dialog)
}

async function chooseReason(dialog: ReturnType<typeof within>, value: string) {
    await userEvent.click(dialog.getByRole("combobox", {name: /Rejection Reason/}))
    await userEvent.click(await within(document.body).findByRole("option", {name: reason(value)}))
}

const submit = (dialog: ReturnType<typeof within>) =>
    userEvent.click(dialog.getByRole("button", {name: i18n.t("approvalsScreen.reject.label")}))

export const RejectForMissingData: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await chooseReason(dialog, "insufficient-information")
        await submit(dialog)
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
        expect(boundary.calls).toEqual([
            {
                name: "ChangeApplicationStatus",
                variables: {
                    tenant_id: TENANT_ID,
                    id: APPLICATION_ID,
                    user_id: "",
                    area_id: STORY_IDS.area,
                    election_event_id: STORY_IDS.event,
                    rejection_reason: "insufficient-information",
                },
                headers: {},
            },
        ])
        expect(args.setRejectDialogOpen).toHaveBeenLastCalledWith(false)
        const message = await within(document.body).findByText(
            i18n.t("approvalsScreen.notifications.rejectSuccess")
        )
        await waitFor(() => expect(message).toBeVisible())
    },
}

export const OtherReasonNeedsAMessage: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await chooseReason(dialog, "other")
        await submit(dialog)
        await expect(
            await dialog.findByText(i18n.t("approvalsScreen.reject.messageRequired"))
        ).toBeVisible()
        expect(boundary.calls).toEqual([])
        expect(args.goBack).not.toHaveBeenCalled()
    },
}

export const OtherReasonWithMessage: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await chooseReason(dialog, "other")
        await userEvent.type(
            dialog.getByRole("textbox", {name: i18n.t("approvalsScreen.reject.message")}),
            "Applicant withdrew"
        )
        await submit(dialog)
        await waitFor(() => expect(args.goBack).toHaveBeenCalledTimes(1))
        expect(boundary.calls[0].variables).toMatchObject({
            rejection_reason: "other",
            rejection_message: "Applicant withdrew",
        })
    },
}

export const RejectionNeedsAReason: Story = {
    play: async () => {
        const dialog = await openDialog()
        await expect(
            dialog.getByRole("button", {name: i18n.t("approvalsScreen.reject.label")})
        ).toBeDisabled()
        expect(boundary.calls).toEqual([])
    },
}

export const CloseWithoutRejecting: Story = {
    play: async ({args}) => {
        const dialog = await openDialog()
        await userEvent.click(dialog.getAllByRole("button")[0])
        expect(args.setRejectDialogOpen).toHaveBeenCalledWith(false)
        expect(args.goBack).not.toHaveBeenCalled()
        expect(boundary.calls).toEqual([])
    },
}

export const ProcessedApplicationCannotBeRejected: Story = {
    args: {status: IApplicationsStatus.ACCEPTED},
    play: async () => {
        expect(within(document.body).queryByRole("dialog")).not.toBeInTheDocument()
    },
}

export const RejectionFailureKeepsTheDialogOpen: Story = {
    args: {failure: true},
    play: async ({args}) => {
        const dialog = await openDialog()
        await chooseReason(dialog, "insufficient-information")
        await submit(dialog)
        const message = await within(document.body).findByText(
            i18n.t("approvalsScreen.notifications.rejectError")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(boundary.calls.map(({name}) => name)).toEqual(["ChangeApplicationStatus"])
        expect(args.goBack).not.toHaveBeenCalled()
        expect(args.setRejectDialogOpen).not.toHaveBeenCalled()
    },
}
