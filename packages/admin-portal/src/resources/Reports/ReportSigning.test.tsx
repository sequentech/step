/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {renderHook, waitFor} from "@testing-library/react"
import {
    ApolloClient,
    ApolloLink,
    ApolloProvider,
    InMemoryCache,
    NormalizedCacheObject,
    Observable,
    type Operation,
} from "@apollo/client"
import {EReportType} from "@/types/reports"
import {IPermissions} from "@/types/keycloak"
import {useActionNeedsSignatures} from "@/components/signing/useSigningRule"
import type {ISigningPanelData} from "@/lib/signing/api"
import {SigningAction, SigningRequestStatus} from "@/lib/signing/types"
import {
    heldByReports,
    releasedDocumentId,
    reportSigningAction,
    useReportSignatures,
} from "./ReportSigning"

jest.mock("@sequentech/ui-essentials", () => ({Dialog: () => null}), {virtual: true})
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@/resources/User/DownloadDocument", () => ({DownloadDocument: () => null}))
jest.mock("@/providers/TenantContextProvider", () => ({useTenantStore: () => ["tenant"]}))
let mockGranted: string[] = []
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (_self: boolean, _tenant: string, permission: string) =>
            mockGranted.includes(permission),
    }),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({globalSettings: {}}),
}))

const EVENT_ID = "2a33fce6-73bb-444b-9112-13d1d3a45fbb"

let operations: Operation[]
let client: ApolloClient<NormalizedCacheObject>

beforeEach(() => {
    mockGranted = [IPermissions.SIGNING_RULES_READ]
    operations = []
    client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    operations.push(operation)
                    observer.next({
                        data: {
                            sequent_backend_signing_rule: [
                                {
                                    action: "generate-reports",
                                    requirement: "required",
                                    signatures: 2,
                                    requester_signing: "allowed",
                                    expires_minutes: null,
                                    revision: 1,
                                    updated_by: "user",
                                    updated_by_name: null,
                                    updated_at: "2026-10-01T00:00:00Z",
                                },
                            ],
                        },
                    })
                    observer.complete()
                })
        ),
    })
})

const renderSignatures = (electionEventId: string) =>
    renderHook(() => useReportSignatures(electionEventId), {
        wrapper: ({children}) => <ApolloProvider client={client}>{children}</ApolloProvider>,
    })

describe("useReportSignatures", () => {
    it("reads an event's rules as the signing-rules reader", async () => {
        const {result} = renderSignatures(EVENT_ID)

        await waitFor(() => expect(result.current.known).toBe(true))
        expect(operations.map((operation) => operation.operationName)).toEqual(["GetSigningRules"])
        expect(operations[0].variables).toEqual({electionEventId: EVENT_ID})
        expect(operations[0].getContext().headers).toEqual({
            "x-hasura-role": IPermissions.SIGNING_RULES_READ,
        })
        expect(result.current.needs(EReportType.PARTICIPATION_REPORT)).toBe(2)
    })

    // The tenant's Reports list has no event: Hasura refuses the query for its missing uuid.
    it("reads no rules without an election event", async () => {
        const {result} = renderSignatures("")

        // Let a query that should not run have its turn.
        await new Promise((settle) => setTimeout(settle, 50))
        expect(operations).toEqual([])
        expect(result.current.known).toBe(false)
        expect(result.current.needs(EReportType.PARTICIPATION_REPORT)).toBeNull()
    })

    it("reads no rules without the permission", async () => {
        mockGranted = []
        const {result} = renderSignatures(EVENT_ID)

        await new Promise((settle) => setTimeout(settle, 50))
        expect(operations).toEqual([])
        expect(result.current.known).toBe(false)
    })
})

describe("report signing actions", () => {
    it("signs each report type where it is produced", () => {
        expect(reportSigningAction(EReportType.ELECTORAL_RESULTS)).toBe(
            SigningAction.GenerateElectionReturns
        )
        expect(reportSigningAction(EReportType.INITIALIZATION_REPORT)).toBe(
            SigningAction.GenerateReports
        )
        expect(reportSigningAction(EReportType.PARTICIPATION_REPORT)).toBe(
            SigningAction.GenerateReports
        )
        expect(reportSigningAction(EReportType.CREDENTIALS)).toBeNull()
        // The tally produces the election returns; the Reports tab holds only the participation report.
        expect(heldByReports(EReportType.PARTICIPATION_REPORT)).toBe(true)
        expect(heldByReports(EReportType.ELECTORAL_RESULTS)).toBe(false)
    })

    it("counts each report's signatures from its action's rule", async () => {
        const {result} = renderSignatures(EVENT_ID)

        await waitFor(() => expect(result.current.known).toBe(true))
        expect(result.current.needs(EReportType.INITIALIZATION_REPORT)).toBe(2)
        // No rule for the election returns: off.
        expect(result.current.needs(EReportType.ELECTORAL_RESULTS)).toBe(0)
        // A report that takes no signatures.
        expect(result.current.needs(EReportType.CREDENTIALS)).toBeNull()
    })

    it("releases a report document only from an executed request", () => {
        const panel = (status: SigningRequestStatus, result: Record<string, unknown> | null) =>
            ({request: {status, execution_result: result}}) as unknown as ISigningPanelData

        expect(releasedDocumentId(panel(SigningRequestStatus.Executed, {document_id: "doc"}))).toBe(
            "doc"
        )
        expect(
            releasedDocumentId(panel(SigningRequestStatus.Executed, {document_id: 7}))
        ).toBeNull()
        expect(releasedDocumentId(panel(SigningRequestStatus.Executed, null))).toBeNull()
        expect(
            releasedDocumentId(panel(SigningRequestStatus.Completed, {document_id: "doc"}))
        ).toBeNull()
    })
})

describe("useActionNeedsSignatures", () => {
    const renderNeeds = (electionEventId: string, action: SigningAction, enabled?: boolean) =>
        renderHook(() => useActionNeedsSignatures(electionEventId, action, enabled), {
            wrapper: ({children}) => <ApolloProvider client={client}>{children}</ApolloProvider>,
        })

    it("says whether the event's rule for an action is required", async () => {
        const required = renderNeeds(EVENT_ID, SigningAction.GenerateReports)
        await waitFor(() => expect(required.result.current).toBe(true))
        expect(operations[0].getContext().headers).toEqual({
            "x-hasura-role": IPermissions.SIGNING_RULES_READ,
        })

        const off = renderNeeds(EVENT_ID, SigningAction.TransmitResults)
        await waitFor(() => expect(off.result.current).toBe(false))
    })

    it("does not know without an event, the permission, or while disabled", async () => {
        const noEvent = renderNeeds("", SigningAction.GenerateReports)
        const disabled = renderNeeds(EVENT_ID, SigningAction.GenerateReports, false)
        mockGranted = []
        const unreadable = renderNeeds(EVENT_ID, SigningAction.GenerateReports)

        await new Promise((settle) => setTimeout(settle, 50))
        expect(operations).toEqual([])
        expect(noEvent.result.current).toBeNull()
        expect(disabled.result.current).toBeNull()
        expect(unreadable.result.current).toBeNull()
    })
})
