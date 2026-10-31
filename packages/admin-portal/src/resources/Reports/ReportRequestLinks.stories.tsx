// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {fakeApi, makePanel, REQUEST_ID, CODE} from "@/components/signing/__stories__/fixtures"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {IPermissions} from "@/types/keycloak"
import {ReportRequestLinks} from "./ReportSigning"

interface Scenario {
    variant: "button" | "menu"
    documentId: string
    status: SigningRequestStatus
}
let api: ReturnType<typeof fakeApi>
let boundary: ReturnType<typeof graphqlBoundary>
let resources: ReturnType<typeof resourceBoundary>
const meta = {
    title: "Admin/Reports/ReportRequestLinks",
    component: ReportRequestLinks,
    args: {variant: "button", documentId: "result-document", status: SigningRequestStatus.Waiting},
    beforeEach: async ({args}) => {
        api = fakeApi(
            await makePanel({action: SigningAction.GenerateElectionReturns, status: args.status})
        )
        resources = resourceBoundary({sequent_backend_election: [], sequent_backend_area: []})
        boundary = graphqlBoundary({
            GetHeldReportRequests: () => ({
                data: {
                    signingHeldReportRequests: {
                        requests: [
                            {
                                request_id: REQUEST_ID,
                                code: CODE,
                                report_type: "ELECTORAL_RESULTS",
                                election_id: STORY_IDS.election,
                                area_id: null,
                                report_id: null,
                                results_event_id: "results",
                                tally_session_id: "tally",
                                results_document_id: "result-document",
                                status: args.status,
                            },
                        ],
                    },
                },
            }),
        })
    },
    render: ({variant, documentId}) => (
        <AdminStoryProvider
            boundary={boundary}
            dataProvider={resources.provider}
            roles={[IPermissions.SIGN_GENERATE_ELECTION_RETURNS]}
            signingApi={api}
        >
            <ReportRequestLinks
                electionEventId={EVENT_ID}
                reportType="ELECTORAL_RESULTS"
                resultsDocumentId={documentId}
                variant={variant}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>
export const OpensTheWaitingReport: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(
            await within(canvasElement).findByRole("button", {
                name: i18n.t("signing.results.openRequest"),
            })
        )
        await waitFor(() => expect(api.getRequest).toHaveBeenCalledWith(REQUEST_ID))
        await expect(await within(document.body).findByTestId("signing-code")).toHaveTextContent(
            CODE
        )
    },
}
export const TallyMenuOpensTheMatchingDocument: Story = {
    args: {variant: "menu"},
    play: async ({canvasElement}) => {
        await userEvent.click(
            await within(canvasElement).findByRole("menuitem", {name: new RegExp(CODE)})
        )
        await waitFor(() => expect(api.getRequest).toHaveBeenCalledWith(REQUEST_ID))
    },
}
export const OtherResultDocumentsHaveNoLink: Story = {
    args: {documentId: "other-document"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(boundary.calls).toHaveLength(1))
        expect(within(canvasElement).queryAllByRole("button")).toEqual([])
    },
}
export const ReleasedReportsHaveNoWaitingLink: Story = {
    args: {status: SigningRequestStatus.Executed},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(boundary.calls).toHaveLength(1))
        expect(within(canvasElement).queryAllByRole("button")).toEqual([])
    },
}
