// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Reports whose action needs signatures: the list's Signatures column, the
// generate dialog that says the report waits for them, the report task's
// signing request, and what an executed request offers (download, print,
// transmit). The election returns and the initialization report are held by
// the tally, per Post (and country); the Reports tab holds the participation
// report of a Post.
import React, {useContext, useEffect, useMemo, useState} from "react"
import {gql, useApolloClient, useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Alert, Box, Button, MenuItem, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EReportType} from "@/types/reports"
import {GET_SIGNING_RULES} from "@/queries/SigningSettings"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import type {
    FetchDocumentQuery,
    GetTaskByIdQuery,
    SendTransmissionPackageMutation,
    CreateTransmissionPackageMutation,
} from "@/gql/graphql"
import type {ISigningPanelData} from "@/lib/signing/api"
import {
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    type ISigningRuleRow,
} from "@/lib/signing/types"
import {useNotify} from "react-admin"
import {useNavigate} from "react-router-dom"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useOptionalSigningRequest} from "@/hooks/useSignedAction"
import {useScopeNames} from "@/resources/ElectionEvent/Signatures/useSigningSettings"
import {CREATE_TRANSMISSION_PACKAGE} from "@/queries/CreateTransmissionPackage"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import type {IMiruTransmissionPackageData} from "@/types/miru"
import {SEND_TRANSMISSION_PACKAGE} from "@/queries/SendTransmissionPackage"
import {signedView} from "@/lib/signing/request"
import {ruleOf} from "@/resources/ElectionEvent/Signatures/signingSettings"
import {DownloadDocument} from "@/resources/User/DownloadDocument"

export const GET_HELD_REPORT_REQUESTS = gql`
    query GetHeldReportRequests($electionEventId: uuid!) {
        signingHeldReportRequests(election_event_id: $electionEventId) {
            requests
        }
    }
`

/** The protected action of a report type, where it is signed: its rule says whether it needs signatures. */
export const reportSigningAction = (reportType: string): SigningAction | null => {
    switch (reportType) {
        case EReportType.ELECTORAL_RESULTS:
            return SigningAction.GenerateElectionReturns
        case EReportType.INITIALIZATION_REPORT:
        case EReportType.PARTICIPATION_REPORT:
            return SigningAction.GenerateReports
        default:
            return null
    }
}

/** Whether the Reports tab generates a report type to be signed (the others are signed by the tally). */
export const heldByReports = (reportType: string): boolean =>
    reportType === EReportType.PARTICIPATION_REPORT

/** How many signatures a report type needs: a number, 0 when off, null when it takes none or the rules can't be read. */
export type ReportSignatures = number | null

/**
 * The signatures each report type needs, read when the viewer may read the
 * signing rules; `known` is false otherwise (the report then generates as
 * before, and its task says whether it waits for signatures).
 */
export function useReportSignatures(electionEventId: string) {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canRead = auth.isAuthorized(true, tenantId, IPermissions.SIGNING_RULES_READ)
    const {data} = useQuery<{sequent_backend_signing_rule: ISigningRuleRow[]}>(GET_SIGNING_RULES, {
        variables: {electionEventId},
        context: {headers: {"x-hasura-role": IPermissions.SIGNING_RULES_READ}},
        // The tenant's report list has no event, so no rules to read.
        skip: !canRead || !electionEventId,
    })
    const rules = data?.sequent_backend_signing_rule
    return useMemo(
        () => ({
            known: !!rules,
            needs: (reportType: string): ReportSignatures => {
                const action = reportSigningAction(reportType)
                if (!action || !rules) return null
                const rule = ruleOf(action, rules)
                return rule.requirement === SigningRequirement.Required ? rule.signatures : 0
            },
        }),
        [rules]
    )
}

/** The Signatures column: "Needs n", "Off", or a dash for a report that takes none. */
export const ReportSignaturesCell: React.FC<{needs: ReportSignatures}> = ({needs}) => {
    const {t} = useTranslation()
    if (needs === null) return <>-</>
    return <>{needs > 0 ? t("signing.results.needs", {n: needs}) : t("signing.results.off")}</>
}

export interface IGenerateSignedReportDialogProps {
    open: boolean
    title: string
    /** The Post the report is of. */
    post: string
    /** Signatures it needs. */
    needs: number
    onClose: (generate: boolean) => void
}

/** "{Post}: the document is generated now. It can be printed and transmitted once n people have signed it." */
export const GenerateSignedReportDialog: React.FC<IGenerateSignedReportDialogProps> = ({
    open,
    title,
    post,
    needs,
    onClose,
}) => {
    const {t} = useTranslation()
    return (
        <Dialog
            variant="info"
            open={open}
            title={title}
            ok={String(t("reportsScreen.actions.generate"))}
            cancel={String(t("common.label.cancel"))}
            handleClose={(generate: boolean) => onClose(generate)}
        >
            <Typography variant="body2">
                {t("signing.reports.generateNotice", {post, n: needs})}
            </Typography>
        </Dialog>
    )
}

/** The signing request a report task started, once the task says so. */
export function useReportTaskSigningRequest(taskId: string | null): string | null {
    const {globalSettings} = useContext(SettingsContext)
    const {data} = useQuery<GetTaskByIdQuery>(GET_TASK_BY_ID, {
        variables: {task_id: taskId},
        skip: !taskId,
        pollInterval: globalSettings.QUERY_FAST_POLL_INTERVAL_MS,
    })
    const annotations = data?.sequent_backend_tasks_execution?.[0]?.annotations as
        | {signing_request?: {id?: string} | null}
        | null
        | undefined
    return annotations?.signing_request?.id ?? null
}

/** The report document an executed request released. */
export const releasedDocumentId = (data: ISigningPanelData): string | null => {
    if (data.request.status !== SigningRequestStatus.Executed) return null
    const id = data.request.execution_result?.document_id
    return typeof id === "string" ? id : null
}

export interface IReportCompletionActionsProps {
    data: ISigningPanelData
}

/**
 * What a released report offers in its panel: Download signed PDF, Print,
 * and for the election returns Transmit results.
 */
export const ReportCompletionActions: React.FC<IReportCompletionActionsProps> = ({data}) => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [downloading, setDownloading] = useState<string | null>(null)
    const navigate = useNavigate()
    const signing = useOptionalSigningRequest()
    const tally = useElectionEventTallyStore()
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canTransmit = [IPermissions.MIRU_CREATE, IPermissions.MIRU_SEND].some((permission) =>
        auth.isAuthorized(true, tenantId, permission)
    )
    const {requests} = useHeldReportRequests(
        data.request.election_event_id,
        canTransmit &&
            data.request.action === SigningAction.GenerateElectionReturns &&
            data.request.status === SigningRequestStatus.Executed
    )
    const target = requests.find(
        (request) =>
            request.request_id === data.request.id &&
            request.election_id === data.request.election_id &&
            request.area_id === data.request.area_id
    )
    const notify = useNotify()
    const [openingTransmission, setOpeningTransmission] = useState(false)
    const [transmissionError, setTransmissionError] = useState(false)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [createPackage] = useMutation<CreateTransmissionPackageMutation>(
        CREATE_TRANSMISSION_PACKAGE,
        {
            context: {headers: {"x-hasura-role": IPermissions.MIRU_CREATE}},
        }
    )
    const showTransmission = (packageData: IMiruTransmissionPackageData, tallyId: string) => {
        signing?.close()
        if (packageData.signing_request?.id && signing) {
            signing.open(packageData.signing_request.id, {eventId: data.request.election_event_id})
            return
        }
        tally.setCreatingFlag(null)
        tally.setSelectedTallySessionData(packageData)
        tally.setTallyId(tallyId)
        tally.setElectionEventIdFlag(data.request.election_event_id)
        navigate(`/sequent_backend_election_event/${data.request.election_event_id}?tabId=tally`)
    }
    const goToTransmission = async () => {
        if (!target?.tally_session_id || !target.election_id || !target.area_id) return
        const tallyId = target.tally_session_id
        setOpeningTransmission(true)
        setTransmissionError(false)
        try {
            if (target.transmission_package) {
                showTransmission(target.transmission_package, tallyId)
                return
            }
            if (!auth.isAuthorized(true, tenantId, IPermissions.MIRU_CREATE)) {
                setTransmissionError(true)
                return
            }
            const widget = addWidget(ETasksExecution.CREATE_TRANSMISSION_PACKAGE, undefined)
            const response = await createPackage({
                variables: {
                    electionEventId: data.request.election_event_id,
                    electionId: target.election_id,
                    tallySessionId: tallyId,
                    areaId: target.area_id,
                    force: false,
                },
            })
            const taskId = response.data?.create_transmission_package?.task_execution?.id
            if (
                response.errors?.length ||
                response.data?.create_transmission_package?.error_msg ||
                !taskId
            ) {
                updateWidgetFail(widget.identifier)
                setTransmissionError(true)
                return
            }
            setWidgetTaskId(widget.identifier, taskId, () => {
                client
                    .query<{signingHeldReportRequests: {requests: IHeldReportRequest[]}}>({
                        query: GET_HELD_REPORT_REQUESTS,
                        variables: {electionEventId: data.request.election_event_id},
                        context: {headers: {"x-hasura-role": IPermissions.MIRU_CREATE}},
                        fetchPolicy: "network-only",
                    })
                    .then(({data: refreshed}) => {
                        const packageData = refreshed.signingHeldReportRequests.requests.find(
                            (request) => request.request_id === target.request_id
                        )?.transmission_package
                        if (packageData) showTransmission(packageData, tallyId)
                        else setTransmissionError(true)
                    })
                    .catch(() => setTransmissionError(true))
            })
            notify(t("miruExport.create.success"), {type: "success"})
        } catch {
            setTransmissionError(true)
        } finally {
            setOpeningTransmission(false)
        }
    }
    const documentId = releasedDocumentId(data)
    if (!documentId) return null
    const electionEventId = data.request.election_event_id
    const print = async () => {
        const {data: fetched} = await client.query<FetchDocumentQuery>({
            query: FETCH_DOCUMENT,
            variables: {electionEventId, documentId},
            fetchPolicy: "network-only",
        })
        const url = fetched?.fetchDocument?.url
        // The browser's PDF viewer prints it.
        if (url) window.open(url, "_blank", "noopener")
    }
    return (
        <>
            <Button variant="contained" onClick={() => setDownloading(documentId)}>
                {t("signing.results.downloadSigned")}
            </Button>
            {/* A password-protected report opens through its download's password flow. */}
            {data.request.execution_result?.protected === true ? null : (
                <Button variant="outlined" onClick={() => void print()}>
                    {t("signing.results.print")}
                </Button>
            )}
            {data.request.action === SigningAction.GenerateElectionReturns && canTransmit ? (
                <Button
                    variant="outlined"
                    disabled={!target?.tally_session_id || openingTransmission}
                    onClick={() => void goToTransmission()}
                >
                    {t("signing.results.transmit")}
                </Button>
            ) : null}
            {transmissionError ? (
                <Alert severity="error">{t("miruExport.create.error")}</Alert>
            ) : null}
            {downloading ? (
                <DownloadDocument
                    documentId={downloading}
                    electionEventId={electionEventId}
                    fileName={null}
                    showReportPasswordDialog
                    onDownload={() => setDownloading(null)}
                />
            ) : null}
        </>
    )
}

/** Completion action available when a transmission request is reopened. */
export const TransmissionCompletionActions: React.FC<IReportCompletionActionsProps> = ({data}) => {
    const {t} = useTranslation()
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const notify = useNotify()
    const [confirm, setConfirm] = useState(false)
    const [sent, setSent] = useState(false)
    const [send, {loading}] = useMutation<SendTransmissionPackageMutation>(
        SEND_TRANSMISSION_PACKAGE,
        {
            context: {headers: {"x-hasura-role": IPermissions.MIRU_SEND}},
        }
    )
    useEffect(() => {
        setConfirm(false)
        setSent(false)
    }, [data.request.id])
    const target = useMemo(() => {
        try {
            const {subject} = signedView(data)
            const tally = subject.tally_session_id
            const destinations = subject.destinations
            return typeof tally === "string" &&
                tally &&
                Array.isArray(destinations) &&
                destinations.length > 0 &&
                destinations.every((value) => typeof value === "string")
                ? {tally, count: destinations.length}
                : null
        } catch {
            return null
        }
    }, [data])
    const {request} = data
    if (
        request.action !== SigningAction.TransmitResults ||
        request.status !== SigningRequestStatus.Executed ||
        !auth.isAuthorized(true, tenantId, IPermissions.MIRU_SEND) ||
        !target ||
        !request.election_id ||
        !request.area_id
    )
        return null

    const transmit = async () => {
        try {
            const response = await send({
                variables: {
                    electionId: request.election_id,
                    areaId: request.area_id,
                    tallySessionId: target.tally,
                },
            })
            if (response.errors?.length || !response.data?.send_transmission_package?.id) {
                notify(t("miruExport.send.error"), {type: "error"})
                return
            }
            setSent(true)
            notify(t("miruExport.send.success"), {type: "success"})
        } catch {
            notify(t("miruExport.send.error"), {type: "error"})
        }
    }
    return (
        <>
            <Button variant="contained" disabled={loading || sent} onClick={() => setConfirm(true)}>
                {t("signing.results.sendTo", {count: target.count})}
            </Button>
            <Dialog
                variant="info"
                open={confirm}
                title={String(t("tally.transmissionPackage.actions.send.dialog.title"))}
                ok={String(t("tally.transmissionPackage.actions.send.dialog.confirm"))}
                cancel={String(t("tally.transmissionPackage.actions.send.dialog.cancel"))}
                handleClose={(confirmed: boolean) => {
                    setConfirm(false)
                    if (confirmed) void transmit()
                }}
            >
                {t("tally.transmissionPackage.actions.send.dialog.description", {
                    name: data.area_name ?? request.area_id,
                })}
            </Dialog>
        </>
    )
}

export interface IHeldReportRequest {
    request_id: string
    code: string
    report_type: string
    election_id: string | null
    area_id: string | null
    report_id: string | null
    results_event_id: string | null
    tally_session_id: string | null
    results_document_id?: string | null
    transmission_package?: IMiruTransmissionPackageData | null
    status: SigningRequestStatus
}

/** Reads safe report references with a permission the viewer already holds. */
export function useHeldReportRequests(electionEventId: string, enabled = true) {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const role = [
        IPermissions.SIGNING_REQUESTS_READ,
        IPermissions.SIGN_GENERATE_ELECTION_RETURNS,
        IPermissions.SIGN_GENERATE_REPORTS,
        IPermissions.REPORT_READ,
        IPermissions.MIRU_CREATE,
        IPermissions.MIRU_SEND,
    ].find((permission) => auth.isAuthorized(true, tenantId, permission))
    const {data, error} = useQuery<{signingHeldReportRequests: {requests: IHeldReportRequest[]}}>(
        GET_HELD_REPORT_REQUESTS,
        {
            variables: {electionEventId},
            context: role ? {headers: {"x-hasura-role": role}} : undefined,
            skip: !enabled || !role || !electionEventId,
            fetchPolicy: "network-only",
            pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )
    return {requests: data?.signingHeldReportRequests?.requests ?? [], error}
}

export const ReportRequestLinks: React.FC<{
    electionEventId: string
    reportType: string
    electionId?: string | null
    reportId?: string | null
    resultsEventId?: string | null
    resultsDocumentId?: string | null
    areaId?: string | null
    variant?: "button" | "menu"
    onOpen?: () => void
}> = ({
    electionEventId,
    reportType,
    electionId,
    reportId,
    resultsEventId,
    resultsDocumentId,
    areaId,
    variant = "button",
    onOpen,
}) => {
    const {t} = useTranslation()
    const signing = useOptionalSigningRequest()
    const {requests, error} = useHeldReportRequests(
        electionEventId,
        !!signing && reportSigningAction(reportType) !== null
    )
    const {postName, countryName} = useScopeNames(electionEventId, !!signing && requests.length > 0)
    if (!signing) return null
    if (error) return <Alert severity="error">{t("signing.waiting.loadError")}</Alert>
    const held = requests.filter(
        (request) =>
            [SigningRequestStatus.Waiting, SigningRequestStatus.Completed].includes(
                request.status
            ) &&
            request.report_type === reportType &&
            (!electionId || request.election_id === electionId) &&
            (!reportId || !request.report_id || request.report_id === reportId) &&
            (!resultsEventId || request.results_event_id === resultsEventId) &&
            (!resultsDocumentId || request.results_document_id === resultsDocumentId) &&
            (areaId === undefined || request.area_id === areaId)
    )
    return (
        <>
            {held.map((request) => {
                const open = () => {
                    signing.open(request.request_id, {eventId: electionEventId})
                    onOpen?.()
                }
                const label = [
                    request.code,
                    request.election_id ? postName(request.election_id) : null,
                    request.area_id ? countryName(request.area_id) : null,
                ]
                    .filter(Boolean)
                    .join(" · ")
                return variant === "menu" ? (
                    <MenuItem key={request.request_id} onClick={open}>
                        {t("signing.results.openRequest")} · {label}
                    </MenuItem>
                ) : (
                    <Box
                        key={request.request_id}
                        sx={{display: "flex", flexWrap: "wrap", alignItems: "center", gap: 1}}
                    >
                        <Button size="small" onClick={open}>
                            {t("signing.results.openRequest")}
                        </Button>
                        <Typography variant="caption">
                            <span>{request.code}</span> · {label.slice(request.code.length + 3)}
                        </Typography>
                    </Box>
                )
            })}
        </>
    )
}
