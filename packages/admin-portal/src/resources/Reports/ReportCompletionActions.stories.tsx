// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {documentHandlers, documentUrl, recordDownloads} from "@/__stories__/downloads"
import type {RecordedDownload} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {makePanel} from "@/components/signing/__stories__/fixtures"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {ReportCompletionActions} from "./ReportSigning"

const TALLY_ID = "77777777-7777-4777-8777-777777777770"
const RELEASED_ID = "66666666-6666-4666-8666-666666666666"

interface Scenario {
    status: SigningRequestStatus
    action: SigningAction
    /** The released report is wrapped in its password. */
    protected?: boolean
}

let boundary: ReturnType<typeof graphqlBoundary>
let panel: ISigningPanelData
let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Reports/ReportCompletionActions",
    component: ReportCompletionActions,
    args: {status: SigningRequestStatus.Executed, action: SigningAction.GenerateElectionReturns},
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary({
            ...documentHandlers({[RELEASED_ID]: {name: "report.pdf"}}),
            GetHeldReportRequests: () => ({
                data: {
                    signingHeldReportRequests: {
                        requests: [
                            {
                                request_id: panel.request.id,
                                code: panel.request.code,
                                report_type: "ELECTORAL_RESULTS",
                                election_id: panel.request.election_id,
                                area_id: panel.request.area_id,
                                report_id: null,
                                results_event_id: null,
                                tally_session_id: TALLY_ID,
                                status: panel.request.status,
                            },
                        ],
                    },
                },
            }),
            SendTransmissionPackage: () => ({data: {send_transmission_package: {id: TALLY_ID}}}),
        })
        await boundary.ready
        const made = await makePanel({
            action: args.action,
            status: args.status,
            required: 3,
            ...(args.action === SigningAction.TransmitResults
                ? {
                      subject: {
                          tally_session_id: TALLY_ID,
                          package_sha256: "00".repeat(32),
                          eml_sha256: "11".repeat(32),
                          destinations: ["east", "west"],
                      },
                  }
                : {}),
        })
        panel = {
            ...made,
            request: {
                ...made.request,
                document_sha256:
                    args.action === SigningAction.TransmitResults
                        ? "11".repeat(32)
                        : made.request.document_sha256,
                execution_result:
                    args.status === SigningRequestStatus.Executed
                        ? {document_id: RELEASED_ID, protected: !!args.protected}
                        : null,
            },
        }
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: () => (
        <AdminStoryProvider
            boundary={boundary}
            roles={[
                IPermissions.MIRU_CREATE,
                IPermissions.MIRU_SEND,
                IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
            ]}
        >
            <ReportCompletionActions data={panel} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const SignedReturnsOfferDownloadPrintAndTransmit: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("button", {name: i18n.t("signing.results.downloadSigned")})
        )
        await waitFor(() =>
            expect(downloads).toEqual([{name: "report.pdf", href: documentUrl(RELEASED_ID)}])
        )
        const open = fn()
        const original = window.open
        window.open = open as unknown as typeof window.open
        try {
            await userEvent.click(
                canvas.getByRole("button", {name: i18n.t("signing.results.print")})
            )
            await waitFor(() =>
                expect(open).toHaveBeenCalledWith(documentUrl(RELEASED_ID), "_blank", "noopener")
            )
        } finally {
            window.open = original
        }
        await expect(
            canvas.getByRole("button", {name: i18n.t("signing.results.transmit")})
        ).toBeVisible()
        expect(
            boundary.calls
                .filter(({name}) => name === "FetchDocument")
                .map(({variables}) => variables)
        ).toContainEqual({electionEventId: EVENT_ID, documentId: RELEASED_ID})
    },
}

export const OtherSignedReportsAreNotTransmitted: Story = {
    args: {action: SigningAction.GenerateReports},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("button", {name: i18n.t("signing.results.downloadSigned")})
        expect(canvas.queryByRole("button", {name: i18n.t("signing.results.transmit")})).toBeNull()
    },
}

export const NothingBeforeTheRequestRan: Story = {
    args: {status: SigningRequestStatus.Completed},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryAllByRole("button")).toEqual([])
    },
}

export const AProtectedReportIsNotPrintedDirectly: Story = {
    args: {protected: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("button", {name: i18n.t("signing.results.downloadSigned")})
        // Opened through its download's password flow instead.
        expect(canvas.queryByRole("button", {name: i18n.t("signing.results.print")})).toBeNull()
    },
}
