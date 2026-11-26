// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {IPermissions} from "@/types/keycloak"
import {ManualVerificationDialog} from "./ManualVerification"
import {REQUEST, VOTER} from "./__stories__/manualVerificationStoryFixture"

const makeBoundary = () =>
    graphqlBoundary({
        ManualVerification: () => ({
            data: {
                get_manual_verification_pdf: {
                    document_id: null,
                    status: "awaiting-signatures",
                    signing_request: {id: REQUEST},
                },
            },
        }),
    })

const meta = {
    title: "Admin/User/ManualVerificationDialog",
    component: ManualVerificationDialog,
    args: {
        tenantId: TENANT_ID,
        electionEventId: EVENT_ID,
        voterId: VOTER,
        reachable: true,
        onClose: fn(),
    },
} satisfies Meta<typeof ManualVerificationDialog>
export default meta

let reachableBoundary: ReturnType<typeof makeBoundary>
export const WaitsForApproval: StoryObj<typeof meta> = {
    beforeEach: () => {
        reachableBoundary = makeBoundary()
    },
    decorators: [
        (Story) => (
            <AdminStoryProvider boundary={reachableBoundary}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
    async play({args, canvasElement}) {
        const dialog = await within(canvasElement.ownerDocument.body).findByRole("dialog")
        await userEvent.click(
            within(dialog).getByRole("button", {name: "Request verification document"})
        )
        await waitFor(() => expect(args.onClose).toHaveBeenCalledTimes(1))
        expect(reachableBoundary.calls).toEqual([
            {
                name: "ManualVerification",
                headers: {"x-hasura-role": IPermissions.VOTER_MANUALLY_VERIFY},
                variables: {
                    tenantId: TENANT_ID,
                    electionEventId: EVENT_ID,
                    voterId: VOTER,
                    initiationId: expect.any(String),
                },
            },
        ])
    },
}

let unreachableBoundary: ReturnType<typeof graphqlBoundary>
export const MissingContact: StoryObj<typeof meta> = {
    beforeEach: () => {
        unreachableBoundary = graphqlBoundary({})
    },
    args: {reachable: false},
    decorators: [
        (Story) => (
            <AdminStoryProvider boundary={unreachableBoundary}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
    async play({canvasElement}) {
        const dialog = await within(canvasElement.ownerDocument.body).findByRole("dialog")
        expect(
            within(dialog).getByRole("button", {name: "Request verification document"})
        ).toBeDisabled()
        await waitFor(() => {
            expect(dialog).toHaveAccessibleDescription(
                /do not have an email address or phone number/
            )
            expect(
                within(dialog).getByText(/do not have an email address or phone number/)
            ).toBeVisible()
        })
        expect(unreachableBoundary.calls).toEqual([])
    },
}
