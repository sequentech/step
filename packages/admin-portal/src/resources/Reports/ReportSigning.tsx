// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Reports whose action needs signatures: the list's Signatures column, the
// generate dialog that says the report waits for them, the report task's
// signing request, and what an executed request offers (download, print,
// transmit). The election returns and the initialization report are held by
// the tally, per Post (and country); the Reports tab holds the participation
// report of a Post.
import React, {useContext, useMemo, useState} from "react"
import {useApolloClient, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Button, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {AuthContext} from "@/providers/AuthContextProvider"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EReportType} from "@/types/reports"
import {GET_SIGNING_RULES} from "@/queries/SigningSettings"
import {GET_TASK_BY_ID} from "@/queries/GetTaskById"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import type {FetchDocumentQuery, GetTaskByIdQuery} from "@/gql/graphql"
import type {ISigningPanelData} from "@/lib/signing/api"
import {
    SigningAction,
    SigningRequestStatus,
    SigningRequirement,
    type ISigningRuleRow,
} from "@/lib/signing/types"
import {ruleOf} from "@/resources/ElectionEvent/Signatures/signingSettings"
import {DownloadDocument} from "@/resources/User/DownloadDocument"

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

/** Where the election returns are transmitted: the event's Tally tab. */
const goToTransmission = (electionEventId: string) => {
    const url = new URL(
        `/sequent_backend_election_event/${electionEventId}`,
        window.location.origin
    )
    url.searchParams.set("tabId", "tally")
    window.location.assign(url.toString())
}

/**
 * What a released report offers in its panel: Download signed PDF, Print,
 * and for the election returns Transmit results.
 */
export const ReportCompletionActions: React.FC<IReportCompletionActionsProps> = ({data}) => {
    const {t} = useTranslation()
    const client = useApolloClient()
    const [downloading, setDownloading] = useState<string | null>(null)
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
            {data.request.action === SigningAction.GenerateElectionReturns ? (
                <Button variant="outlined" onClick={() => goToTransmission(electionEventId)}>
                    {t("signing.results.transmit")}
                </Button>
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
