/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import "@testing-library/jest-dom"
import englishTranslation from "@/translations/en"
import spanishTranslation from "@/translations/es"
import catalanTranslation from "@/translations/cat"
import basqueTranslation from "@/translations/eu"
import frenchTranslation from "@/translations/fr"
import galicianTranslation from "@/translations/gl"
import dutchTranslation from "@/translations/nl"
import tagalogTranslation from "@/translations/tl"
import {fireEvent, render, renderHook, screen, waitFor} from "@testing-library/react"
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
    TransmissionCompletionActions,
    ReportRequestLinks,
    ReportCompletionActions,
} from "./ReportSigning"

jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        Dialog: ({
            open,
            ok,
            handleClose,
        }: {
            open: boolean
            ok: string
            handleClose: (confirmed: boolean) => void
        }) =>
            open
                ? require("react").createElement("button", {onClick: () => handleClose(true)}, ok)
                : null,
    }),
    {virtual: true}
)
const mockNotify = jest.fn()
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
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
const mockOpenRequest = jest.fn()
jest.mock("@/hooks/useSignedAction", () => ({
    useOptionalSigningRequest: () => ({open: mockOpenRequest, close: mockCloseRequest}),
}))
jest.mock("@/resources/ElectionEvent/Signatures/useSigningSettings", () => ({
    useScopeNames: () => ({postName: (id: string) => id, countryName: (id: string) => id}),
}))
const mockNavigate = jest.fn()
const mockCloseRequest = jest.fn()
const mockSetTally = jest.fn()
const mockSetPackage = jest.fn()
let mockPackage: Record<string, unknown> | undefined
const mockSetWidgetTask = jest.fn()
const mockWidgetFail = jest.fn()
jest.mock("@/providers/WidgetsContextProvider", () => ({
    useWidgetStore: () => [
        jest.fn(() => ({identifier: "create-package"})),
        mockSetWidgetTask,
        mockWidgetFail,
    ],
}))
jest.mock("react-router-dom", () => ({useNavigate: () => mockNavigate}))
jest.mock("@/providers/ElectionEventTallyProvider", () => ({
    useElectionEventTallyStore: () => ({
        setTallyId: mockSetTally,
        setSelectedTallySessionData: mockSetPackage,
        setElectionEventIdFlag: jest.fn(),
        setCreatingFlag: jest.fn(),
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
    mockNotify.mockClear()
    mockOpenRequest.mockClear()
    mockNavigate.mockClear()
    mockCloseRequest.mockClear()
    mockSetTally.mockClear()
    mockSetPackage.mockClear()
    mockSetWidgetTask.mockClear()
    mockWidgetFail.mockClear()
    mockPackage = {
        election_id: "post-a",
        area_id: "country-a",
        documents: [],
        servers: [],
        logs: [],
        threshold: 2,
    }
    client = new ApolloClient({
        cache: new InMemoryCache({addTypename: false}),
        link: new ApolloLink(
            (operation) =>
                new Observable((observer) => {
                    operations.push(operation)
                    if (operation.operationName === "GetHeldReportRequests") {
                        observer.next({
                            data: {
                                signingHeldReportRequests: {
                                    requests: [
                                        {
                                            request_id: "held-a",
                                            code: "CODE-A",
                                            report_type: "ELECTORAL_RESULTS",
                                            election_id: "post-a",
                                            area_id: "country-a",
                                            report_id: null,
                                            results_event_id: "results-a",
                                            tally_session_id: "tally-a",
                                            results_document_id: "document-a",
                                            status: "waiting",
                                        },
                                        {
                                            request_id: "held-b",
                                            code: "CODE-B",
                                            report_type: "ELECTORAL_RESULTS",
                                            election_id: "post-b",
                                            area_id: "country-b",
                                            report_id: null,
                                            results_event_id: "results-b",
                                            tally_session_id: "tally-b",
                                            results_document_id: "document-b",
                                            status: "waiting",
                                        },
                                        {
                                            request_id: "released",
                                            code: "DONE",
                                            report_type: "ELECTORAL_RESULTS",
                                            election_id: "post-a",
                                            area_id: "country-a",
                                            report_id: null,
                                            results_event_id: "results-a",
                                            tally_session_id: "tally-a",
                                            status: "executed",
                                            transmission_package: mockPackage,
                                        },
                                    ],
                                },
                            },
                        })
                        observer.complete()
                        return
                    }
                    if (operation.operationName === "CreateTransmissionPackage") {
                        observer.next({
                            data: {
                                create_transmission_package: {task_execution: {id: "package-task"}},
                            },
                        })
                        observer.complete()
                        return
                    }
                    if (operation.operationName === "SendTransmissionPackage") {
                        observer.next({
                            data: {
                                send_transmission_package: {id: operation.variables.tallySessionId},
                            },
                        })
                        observer.complete()
                        return
                    }
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

// These signer-facing additions must be translated, preserving every substitution.
describe("signing request translations", () => {
    const additions = [
        "protectedActions.footerFirstVersion",
        "panel.configurationVersion",
        "panel.configurationChanges",
        "values.ruleChange",
        "values.ruleChangeFrom",
        "values.ruleNeeds",
        "values.ruleOff",
        "waiting.title",
        "waiting.buttonCount_one",
        "waiting.buttonCount_other",
        "waiting.intro",
        "waiting.close",
        "waiting.empty",
        "waiting.loadError",
        "waiting.signedByYou",
        "notes.afterApproval",
        "notes.afterApprovalValue",
        "notes.keyShare",
        "notes.keyShareChecked",
        "notes.recordedIn",
        "notes.recordedInCeremony",
        "notes.recordedInTally",
    ]
    const textAt = (source: unknown, path: string): string => {
        let value: unknown = source
        for (const part of path.split(".")) {
            if (value === null || typeof value !== "object" || !(part in value)) {
                throw new Error(`Missing translated text: ${path}`)
            }
            value = (value as Record<string, unknown>)[part]
        }
        if (typeof value !== "string") throw new Error(`Not a text: ${path}`)
        return value
    }
    const placeholders = (text: string) => (text.match(/\{\{[^}]+\}\}|\$t\([^)]+\)/g) ?? []).sort()

    it.each([
        ["Spanish", spanishTranslation],
        ["Catalan", catalanTranslation],
        ["Basque", basqueTranslation],
        ["French", frenchTranslation],
        ["Galician", galicianTranslation],
        ["Dutch", dutchTranslation],
        ["Tagalog", tagalogTranslation],
    ] as const)(
        "translates signer guidance in %s without losing substitutions",
        (language, translation) => {
            for (const path of additions) {
                const original = textAt(englishTranslation.translations.signing, path)
                const translated = textAt(translation.translations.signing, path)
                expect(placeholders(translated)).toEqual(placeholders(original))
                // The first label is substitutions only; "was" is also correct Dutch.
                if (
                    path !== "values.ruleChange" &&
                    !(language === "Dutch" && path === "values.ruleChangeFrom")
                ) {
                    expect(translated).not.toBe(original)
                }
            }
        }
    )
})

describe("transmission completion", () => {
    const panel = (post: string, country: string, tally: string): ISigningPanelData => {
        const request = {
            id: "request",
            code: "CODE",
            tenant_id: "tenant",
            election_event_id: EVENT_ID,
            election_id: post,
            area_id: country,
            action: SigningAction.TransmitResults,
            status: SigningRequestStatus.Executed,
            document_sha256: "a".repeat(64),
        }
        const {canonicalJson, SIGNING_DOMAIN} =
            require("@/lib/signing/request") as typeof import("@/lib/signing/request")
        return {
            request: {
                ...request,
                canonical_payload: canonicalJson({
                    domain: SIGNING_DOMAIN,
                    request_id: request.id,
                    code: request.code,
                    tenant_id: request.tenant_id,
                    election_event_id: request.election_event_id,
                    election_id: post,
                    area_id: country,
                    action: request.action,
                    subject: {
                        tally_session_id: tally,
                        eml_sha256: request.document_sha256,
                        destinations: ["east", "west"],
                    },
                }),
            },
        } as unknown as ISigningPanelData
    }
    const show = (data: ISigningPanelData) =>
        render(
            <ApolloProvider client={client}>
                <TransmissionCompletionActions data={data} />
            </ApolloProvider>
        )

    it.each([
        ["post-a", "country-a", "tally-a"],
        ["faculty", "district", "student-tally"],
    ])("sends the signed scope after confirmation for %s", async (post, country, tally) => {
        mockGranted = [IPermissions.MIRU_SEND]
        show(panel(post, country, tally))
        fireEvent.click(screen.getByRole("button", {name: "signing.results.sendTo"}))
        expect(operations).toEqual([])
        fireEvent.click(
            screen.getByRole("button", {
                name: "tally.transmissionPackage.actions.send.dialog.confirm",
            })
        )
        await waitFor(() => expect(operations).toHaveLength(1))
        expect(operations[0].variables).toEqual({
            electionId: post,
            areaId: country,
            tallySessionId: tally,
        })
        expect(operations[0].getContext().headers).toEqual({
            "x-hasura-role": IPermissions.MIRU_SEND,
        })
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("miruExport.send.success", {type: "success"})
        )
    })

    it("offers no send before execution, without permission, or for a mismatched signed scope", () => {
        const data = panel("post", "country", "tally")
        const withoutPermission = show(data)
        expect(screen.queryByRole("button", {name: "signing.results.sendTo"})).toBeNull()
        withoutPermission.unmount()
        mockGranted = [IPermissions.MIRU_SEND]
        const waiting = show({
            ...data,
            request: {...data.request, status: SigningRequestStatus.Waiting},
        })
        expect(screen.queryByRole("button", {name: "signing.results.sendTo"})).toBeNull()
        waiting.unmount()
        show({...data, request: {...data.request, election_id: "other-post"}})
        expect(screen.queryByRole("button", {name: "signing.results.sendTo"})).toBeNull()
        expect(operations).toEqual([])
    })
})

describe("held report request links", () => {
    it.each([
        ["post-a", "country-a", "results-a", "CODE-A", "held-a"],
        ["post-b", "country-b", "results-b", "CODE-B", "held-b"],
    ])(
        "opens only the waiting request of %s and its country",
        async (post, country, results, code, request) => {
            mockGranted = [IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
            render(
                <ApolloProvider client={client}>
                    <ReportRequestLinks
                        electionEventId={EVENT_ID}
                        reportType="ELECTORAL_RESULTS"
                        electionId={post}
                        areaId={country}
                        resultsEventId={results}
                    />
                </ApolloProvider>
            )
            await screen.findByText(code)
            expect(screen.queryByText("DONE")).toBeNull()
            expect(
                screen.getAllByRole("button", {name: "signing.results.openRequest"})
            ).toHaveLength(1)
            fireEvent.click(screen.getByRole("button", {name: "signing.results.openRequest"}))
            expect(mockOpenRequest).toHaveBeenCalledWith(request, {eventId: EVENT_ID})
            expect(operations[0].getContext().headers).toEqual({
                "x-hasura-role": IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
            })
        }
    )

    it("links a tally menu to its exact result document", async () => {
        mockGranted = [IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
        render(
            <ApolloProvider client={client}>
                <ReportRequestLinks
                    electionEventId={EVENT_ID}
                    reportType="ELECTORAL_RESULTS"
                    resultsDocumentId="document-b"
                    variant="menu"
                />
            </ApolloProvider>
        )
        const item = await screen.findByRole("menuitem", {name: /CODE-B/})
        expect(screen.queryByRole("menuitem", {name: /CODE-A/})).toBeNull()
        fireEvent.click(item)
        expect(mockOpenRequest).toHaveBeenCalledWith("held-b", {eventId: EVENT_ID})
    })

    it("reads no held requests without a read or report-signing permission", async () => {
        mockGranted = []
        render(
            <ApolloProvider client={client}>
                <ReportRequestLinks electionEventId={EVENT_ID} reportType="ELECTORAL_RESULTS" />
            </ApolloProvider>
        )
        await new Promise((settle) => setTimeout(settle, 50))
        expect(operations).toEqual([])
    })
})

describe("report transmission entry point", () => {
    const releasedPanel = () =>
        ({
            request: {
                id: "released",
                action: SigningAction.GenerateElectionReturns,
                status: SigningRequestStatus.Executed,
                election_event_id: EVENT_ID,
                election_id: "post-a",
                area_id: "country-a",
                execution_result: {document_id: "signed-pdf"},
            },
        }) as unknown as ISigningPanelData
    const openTransmission = async () => {
        render(
            <ApolloProvider client={client}>
                <ReportCompletionActions data={releasedPanel()} />
            </ApolloProvider>
        )
        const button = screen.getByRole("button", {name: "signing.results.transmit"})
        await waitFor(() => expect(button).not.toBeDisabled())
        fireEvent.click(button)
    }
    it("opens the released report's tally and country transmission wizard", async () => {
        mockGranted = [IPermissions.MIRU_CREATE, IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
        const data = {
            request: {
                id: "released",
                action: SigningAction.GenerateElectionReturns,
                status: SigningRequestStatus.Executed,
                election_event_id: EVENT_ID,
                election_id: "post-a",
                area_id: "country-a",
                execution_result: {document_id: "signed-pdf"},
            },
        } as unknown as ISigningPanelData
        render(
            <ApolloProvider client={client}>
                <ReportCompletionActions data={data} />
            </ApolloProvider>
        )
        const transmit = screen.getByRole("button", {name: "signing.results.transmit"})
        await waitFor(() => expect(transmit).not.toBeDisabled())
        fireEvent.click(transmit)
        expect(mockSetTally).toHaveBeenCalledWith("tally-a")
        expect(mockCloseRequest).toHaveBeenCalledTimes(1)
        expect(mockSetPackage).toHaveBeenCalledWith(
            expect.objectContaining({election_id: "post-a", area_id: "country-a"})
        )
        const target = new URL(mockNavigate.mock.calls[0][0], "https://portal.test")
        expect(target.pathname).toBe(`/sequent_backend_election_event/${EVENT_ID}`)
        expect(Object.fromEntries(target.searchParams)).toEqual({
            tabId: "tally",
        })
    })
    it("opens an existing signing request directly", async () => {
        mockGranted = [IPermissions.MIRU_SEND, IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
        mockPackage = {...mockPackage, signing_request: {id: "transmission-request"}}
        await openTransmission()
        expect(mockOpenRequest).toHaveBeenCalledWith("transmission-request", {eventId: EVENT_ID})
        expect(mockNavigate).not.toHaveBeenCalled()
        expect(
            operations.some(({operationName}) => operationName === "CreateTransmissionPackage")
        ).toBe(false)
    })

    it("creates the exact package and opens its request after the task succeeds", async () => {
        mockGranted = [IPermissions.MIRU_CREATE, IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
        mockPackage = undefined
        await openTransmission()
        await waitFor(() => expect(mockSetWidgetTask).toHaveBeenCalled())
        const create = operations.find(
            ({operationName}) => operationName === "CreateTransmissionPackage"
        )!
        expect(create.variables).toEqual({
            electionEventId: EVENT_ID,
            electionId: "post-a",
            areaId: "country-a",
            tallySessionId: "tally-a",
            force: false,
        })
        expect(create.getContext().headers).toEqual({"x-hasura-role": IPermissions.MIRU_CREATE})
        expect(mockNavigate).not.toHaveBeenCalled()
        expect(mockOpenRequest).not.toHaveBeenCalled()
        mockPackage = {
            election_id: "post-a",
            area_id: "country-a",
            signing_request: {id: "new-transmission"},
        }
        mockSetWidgetTask.mock.calls[0][2]()
        await waitFor(() =>
            expect(mockOpenRequest).toHaveBeenCalledWith("new-transmission", {eventId: EVENT_ID})
        )
        expect(mockSetWidgetTask.mock.calls[0].slice(0, 2)).toEqual([
            "create-package",
            "package-task",
        ])
    })

    it("requires creation permission when no package exists", async () => {
        mockGranted = [IPermissions.MIRU_SEND, IPermissions.SIGN_GENERATE_ELECTION_RETURNS]
        mockPackage = undefined
        await openTransmission()
        expect(await screen.findByRole("alert")).toHaveTextContent("miruExport.create.error")
        expect(
            operations.some(({operationName}) => operationName === "CreateTransmissionPackage")
        ).toBe(false)
        expect(mockNavigate).not.toHaveBeenCalled()
    })
})
