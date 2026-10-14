// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {SaveButton, SimpleForm, Toolbar} from "react-admin"
import {
    EReceiptsPolicy,
    EVoterSigningPolicy,
    EVotingStatus,
    i18n,
    type IElectionEventStatus,
} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {BallotReceiptsInput} from "./BallotReceiptsInput"

interface Scenario {
    canEdit: boolean
    status?: Partial<IElectionEventStatus>
    /** The receipts policy the event was saved with; absent in older events. */
    policy?: EReceiptsPolicy
    /** The form's submit handler, which receives the edited event values. */
    onSubmit: (values: Record<string, unknown>) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Election event/BallotReceiptsInput",
    component: BallotReceiptsInput,
    args: {canEdit: true, onSubmit: fn()},
    beforeEach: async () => {
        boundary = graphqlBoundary({}, {schema: true})
        await boundary.ready
    },
    render: ({onSubmit, policy, ...args}) => (
        <AdminStoryProvider boundary={boundary}>
            <SimpleForm
                record={{
                    id: "11111111-1111-4111-8111-111111111111",
                    presentation: {
                        voter_signing_policy: EVoterSigningPolicy.NO_SIGNATURE,
                        ...(policy ? {receipts: {policy}} : {}),
                    },
                }}
                onSubmit={onSubmit}
                toolbar={
                    <Toolbar>
                        <SaveButton />
                    </Toolbar>
                }
            >
                <BallotReceiptsInput {...args} />
            </SimpleForm>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const text = (key: string) => i18n.t(`electionEventScreen.field.receiptsPolicy.${key}`)
const policyInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("combobox", {name: new RegExp(text("policyLabel"))})

export const EventWithoutTheSetting: Story = {
    play: async ({canvasElement}) => {
        const input = await policyInput(canvasElement)
        await expect(input).toHaveTextContent(text(EReceiptsPolicy.DISABLED))
        expect(input).not.toHaveAttribute("aria-disabled", "true")
        expect(within(canvasElement).getByText(text("helperText"))).toBeVisible()
    },
}

export const TurnOnSignsBallotsToo: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await policyInput(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: text(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX),
            })
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit).toHaveBeenCalledWith(
            expect.objectContaining({
                presentation: {
                    voter_signing_policy: EVoterSigningPolicy.WITH_SIGNATURE,
                    receipts: {policy: EReceiptsPolicy.SIGNED_BY_BALLOT_BOX},
                },
            }),
            expect.anything()
        )
    },
}

export const TurnOffLeavesVoterSigning: Story = {
    args: {policy: EReceiptsPolicy.SIGNED_BY_BALLOT_BOX},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await policyInput(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {
                name: text(EReceiptsPolicy.DISABLED),
            })
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await waitFor(() => expect(args.onSubmit).toHaveBeenCalledTimes(1))
        expect(args.onSubmit).toHaveBeenCalledWith(
            expect.objectContaining({
                presentation: {
                    voter_signing_policy: EVoterSigningPolicy.NO_SIGNATURE,
                    receipts: {policy: EReceiptsPolicy.DISABLED},
                },
            }),
            expect.anything()
        )
    },
}

export const LockedOnceVotingHasStarted: Story = {
    args: {
        policy: EReceiptsPolicy.SIGNED_BY_BALLOT_BOX,
        status: {voting_status: EVotingStatus.OPEN},
    },
    play: async ({canvasElement}) => {
        const input = await policyInput(canvasElement)
        await expect(input).toHaveTextContent(text(EReceiptsPolicy.SIGNED_BY_BALLOT_BOX))
        expect(input).toHaveAttribute("aria-disabled", "true")
        expect(within(canvasElement).getByText(text("lockedHelperText"))).toBeVisible()
    },
}

export const ReadOnly: Story = {
    args: {canEdit: false},
    play: async ({canvasElement}) => {
        await expect(await policyInput(canvasElement)).toHaveAttribute("aria-disabled", "true")
    },
}
