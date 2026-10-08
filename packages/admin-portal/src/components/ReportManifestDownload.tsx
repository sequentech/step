// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useQuery} from "@apollo/client"
import FingerprintIcon from "@mui/icons-material/Fingerprint"
import {useTranslation} from "react-i18next"
import {GetDocumentQuery} from "@/gql/graphql"
import {GET_DOCUMENT} from "@/queries/GetDocument"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {REPORT_MANIFEST_FILE_NAME, reportManifestDocumentId} from "@/lib/reportManifest"
import {DownloaButton} from "./styles/WidgetStyle"

export interface ReportManifestDownloadProps {
    /** The generated report's document. */
    documentId?: string | null
    electionEventId?: string | null
    /** While the report is not generated yet. */
    disabled?: boolean
}

/**
 * Downloads the hash manifest of a generated report. It shows only for a
 * report that was stored with one.
 */
export const ReportManifestDownload: React.FC<ReportManifestDownloadProps> = ({
    documentId,
    electionEventId,
    disabled = false,
}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const [downloading, setDownloading] = useState(false)
    const {data} = useQuery<GetDocumentQuery>(GET_DOCUMENT, {
        variables: {id: documentId, tenantId},
        skip: !documentId || !tenantId || disabled,
    })
    const manifestDocumentId = reportManifestDocumentId(
        data?.sequent_backend_document?.[0]?.annotations
    )
    if (!manifestDocumentId) {
        return null
    }
    return (
        <>
            <DownloaButton
                onClick={() => setDownloading(true)}
                disabled={disabled || downloading}
                label={String(t("tasksScreen.widget.downloadHashManifest"))}
            >
                <FingerprintIcon />
            </DownloaButton>
            {downloading ? (
                <DownloadDocument
                    documentId={manifestDocumentId}
                    electionEventId={electionEventId ?? undefined}
                    fileName={REPORT_MANIFEST_FILE_NAME}
                    onDownload={() => setDownloading(false)}
                />
            ) : null}
        </>
    )
}
