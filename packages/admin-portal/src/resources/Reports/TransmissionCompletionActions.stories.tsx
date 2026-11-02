// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {makePanel} from "@/components/signing/__stories__/fixtures"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {TransmissionCompletionActions} from "./ReportSigning"

interface Scenario {
    status: SigningRequestStatus
    allowed: boolean
    mismatched?: boolean
}
const TALLY_ID = "77777777-7777-4777-8777-777777777777"
let panel: ISigningPanelData
let boundary: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Reports/TransmissionCompletionActions",
    component: TransmissionCompletionActions,
    args: {status: SigningRequestStatus.Executed, allowed: true},
    beforeEach: async ({args}) => {
        const made = await makePanel({
            action: SigningAction.TransmitResults,
            status: args.status,
            required: 2,
            subject: {
                tally_session_id: TALLY_ID,
                package_sha256: "00".repeat(32),
                eml_sha256: "11".repeat(32),
                destinations: ["east", "west"],
            },
        })
        panel = {
            ...made,
            request: {
                ...made.request,
                document_sha256: "11".repeat(32),
                ...(args.mismatched ? {election_id: TALLY_ID} : {}),
            },
        }
        boundary = graphqlBoundary({
            SendTransmissionPackage: () => ({data: {send_transmission_package: {id: TALLY_ID}}}),
        })
    },
    render: ({allowed}) => (
        <AdminStoryProvider boundary={boundary} roles={allowed ? [IPermissions.MIRU_SEND] : []}>
            <TransmissionCompletionActions data={panel} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const ReopenedExecutedRequestSendsItsSignedScope: Story = {
    play: async ({canvasElement}) => {
        const button = within(canvasElement).getByRole("button", {
            name: i18n.t("signing.results.sendTo", {count: 2}),
        })
        await userEvent.click(button)
        expect(boundary.calls).toEqual([])
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(
            within(dialog).getByRole("button", {
                name: i18n.t("tally.transmissionPackage.actions.send.dialog.confirm"),
            })
        )
        await waitFor(() =>
            expect(boundary.calls).toEqual([
                {
                    name: "SendTransmissionPackage",
                    variables: {
                        electionId: panel.request.election_id,
                        areaId: panel.request.area_id,
                        tallySessionId: TALLY_ID,
                    },
                    headers: {"x-hasura-role": IPermissions.MIRU_SEND},
                },
            ])
        )
        await expect(button).toBeDisabled()
    },
}
export const WaitingRequestCannotSend: Story = {
    args: {status: SigningRequestStatus.Waiting},
    play: ({canvasElement}) => expect(within(canvasElement).queryAllByRole("button")).toEqual([]),
}
export const WithoutSendPermission: Story = {
    args: {allowed: false},
    play: ({canvasElement}) => expect(within(canvasElement).queryAllByRole("button")).toEqual([]),
}
export const ScopeMustMatchTheSignedPayload: Story = {
    args: {mismatched: true},
    play: ({canvasElement}) => expect(within(canvasElement).queryAllByRole("button")).toEqual([]),
}
