// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {SigningAction} from "@/lib/signing/types"
import {OperationalApprovalDialog} from "./OperationalApprovalDialog"
import {fakeApi, makePanel, MARIA, ORGANIZATIONS, signedInAs} from "./__stories__/fixtures"

interface Scenario {
    provider: "softhsm-test" | "yubihsm"
    setupPhase: "setup-retry-available" | "setup-recovery-required"
}

let panel: ISigningPanelData
let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Signing/OperationalApprovalDialog",
    component: OperationalApprovalDialog,
    args: {provider: "softhsm-test", setupPhase: "setup-retry-available"},
    beforeEach: async ({args}) => {
        panel = await makePanel({
            action: SigningAction.InitializeVoting,
            signingMode: "hsm-required",
            subject: {},
            required: 2,
        })
        const scope = {
            tenant: panel.request.tenant_id,
            event: panel.request.election_event_id,
            request: panel.request.id,
            official: MARIA.userId,
        }
        boundary = graphqlBoundary({
            OperationalInitializeStatus: () => ({
                data: {
                    operationalInitializeStatus: {
                        __typename: "OperationalInitializeStatusView",
                        scope: {...scope, __typename: "OperationalInitializeScope"},
                        device_profile: args.provider,
                        phase: "unavailable",
                        cancellable_attempt: null,
                    },
                },
            }),
            OperationalInitializeSetupStatus: () => ({
                data: {
                    operationalInitializeSetupStatus: {
                        __typename: "OperationalInitializeSetupView",
                        scope: {...scope, __typename: "OperationalInitializeScope"},
                        phase: args.setupPhase,
                    },
                },
            }),
        })
    },
    render: () => (
        <AdminStoryProvider
            boundary={boundary}
            auth={signedInAs(MARIA)}
            tenantRecord={ORGANIZATIONS.first}
        >
            <OperationalApprovalDialog
                data={panel}
                api={fakeApi(panel) as ISigningApi}
                eligible
                onClose={fn()}
                onRefresh={fn()}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const dialog = async () => {
    const element = await within(document.body).findByRole("dialog", {name: "Review approval"})
    await waitFor(() => expect(element).toBeVisible(), {timeout: 3000})
    return within(element)
}

const unavailableOutsideSigningOrigin = async (view: ReturnType<typeof within>) => {
    // Storybook does not run on an installed local signing origin. Plain
    // HTTP must not even ask the service to report a device or setup phase.
    await waitFor(() =>
        expect(view.getByRole("status")).toHaveTextContent(
            "Signing is unavailable for this request. No approval was submitted here."
        )
    )
    await expect(view.getByText("0 of 2 approvals recorded")).toBeVisible()
    expect(view.queryByRole("button", {name: "Retry signing setup"})).toBeNull()
    expect(view.queryByRole("button", {name: "Set up signing"})).toBeNull()
    expect(view.queryByRole("button", {name: /^Approve$/})).toBeNull()
    expect(view.queryByText(/software test token/i)).toBeNull()
    expect(boundary.calls).toHaveLength(0)
}

export const UnusedSoftHsmSetup: Story = {
    play: async () => {
        const view = await dialog()
        if (window.location.protocol !== "https:") {
            await unavailableOutsideSigningOrigin(view)
            return
        }
        await expect(await view.findByRole("button", {name: "Retry signing setup"})).toBeVisible()
        await expect(view.getByText("Isolated demo signing. Not for production use.")).toBeVisible()
        await expect(
            view.getByText(/uses? a software test token|signatures use a software test token/i)
        ).toBeVisible()
        await expect(view.getByText("0 of 2 approvals recorded")).toBeVisible()
        expect(view.queryByRole("button", {name: /^Approve$/})).toBeNull()
        expect(view.queryByRole("textbox", {name: /password/i})).toBeNull()
    },
}

export const YubiHsmNeedsOperator: Story = {
    args: {provider: "yubihsm", setupPhase: "setup-recovery-required"},
    play: async () => {
        const view = await dialog()
        if (window.location.protocol !== "https:") {
            await unavailableOutsideSigningOrigin(view)
            return
        }
        await waitFor(
            async () => {
                const status = view.getByRole("status")
                await expect(status).toHaveTextContent(
                    "Signing setup needs help from the demo operator. This request is still pending and cannot be signed here."
                )
                await expect(status).toBeVisible()
            },
            {timeout: 3000}
        )
        await expect(view.getByText("Isolated demo signing. Not for production use.")).toBeVisible()
        expect(view.queryByText(/software test token/i)).toBeNull()
        expect(view.queryByRole("button", {name: "Retry signing setup"})).toBeNull()
        expect(view.queryByRole("button", {name: /^Approve$/})).toBeNull()
    },
}
