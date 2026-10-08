// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {useLazyQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Box, Button, Link, Typography} from "@mui/material"
import OpenInNewIcon from "@mui/icons-material/OpenInNew"
import DownloadIcon from "@mui/icons-material/Download"
import {downloadUrl} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {FETCH_DOCUMENT} from "@/queries/FetchDocument"
import {hasGraphQLActionErrorCode} from "@/services/graphqlActionError"
import type {FetchDocumentQuery, FetchDocumentQueryVariables} from "@/gql/graphql"
import type {IBallotBoxSeal} from "@/types/ballotBoxSeal"

export interface SealRecordLinkProps {
    electionEventId: string
    /** A published seal. */
    seal: IBallotBoxSeal
    areaName: string
}

/** The file name a downloaded restricted seal record is saved as: its election and area. */
export const sealRecordFileName = (seal: IBallotBoxSeal): string =>
    `ballot-box-seal-${seal.election_id}-${seal.area_id}.json`

/** What `fetchDocument` answers when the document row is gone. */
const DOCUMENT_NOT_FOUND = "Document not found"

/** Why a restricted record wasn't downloaded. */
enum ERecordDownloadFailure {
    /** The document is gone: retrying never helps, it is an incident. */
    MISSING = "missing",
    /** Anything else: it may work again. */
    ERROR = "error",
}

/**
 * The seal record of a published ballot box (VOTE-FREEZE). A public record
 * (Seal Record Publication policy Public) is a link to the public bucket. A
 * restricted one is a private event document: an administrator with the
 * document download permission downloads it through a presigned URL; others
 * are told it is restricted.
 */
export const SealRecordLink: React.FC<SealRecordLinkProps> = ({
    electionEventId,
    seal,
    areaName,
}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const [failed, setFailed] = useState<ERecordDownloadFailure | null>(null)
    const [fetchDocument, {loading}] = useLazyQuery<
        FetchDocumentQuery,
        FetchDocumentQueryVariables
    >(FETCH_DOCUMENT, {fetchPolicy: "no-cache"})

    if (seal.public_path) {
        return (
            <Link
                href={`${globalSettings.PUBLIC_BUCKET_URL}${seal.public_path.replace(/^\/+/, "")}`}
                target="_blank"
                rel="noopener noreferrer"
                aria-label={t("dashboard.ballotBoxes.openRecord", {area: areaName})}
                sx={{display: "inline-flex", alignItems: "center", gap: 0.5}}
            >
                {t("dashboard.ballotBoxes.column.record")}
                <OpenInNewIcon fontSize="inherit" />
            </Link>
        )
    }
    const documentId = seal.public_document_id ? String(seal.public_document_id) : undefined
    if (!documentId) return <>—</>
    if (!auth.isAuthorized(true, tenantId, IPermissions.DOCUMENT_DOWNLOAD)) {
        return (
            <Typography variant="caption" color="text.secondary">
                {t("dashboard.ballotBoxes.recordRestricted")}
            </Typography>
        )
    }

    const download = async () => {
        setFailed(null)
        try {
            const {data, error} = await fetchDocument({
                variables: {electionEventId, documentId},
            })
            const url = data?.fetchDocument?.url
            if (error || !url) throw error ?? new Error("The seal record has no download URL")
            await downloadUrl(url, sealRecordFileName(seal))
        } catch (error) {
            const missing = hasGraphQLActionErrorCode(error, DOCUMENT_NOT_FOUND)
            console.error(
                missing
                    ? "The seal record's document is missing: a published seal's record must stay"
                    : "Downloading the seal record failed",
                error
            )
            setFailed(missing ? ERecordDownloadFailure.MISSING : ERecordDownloadFailure.ERROR)
        }
    }

    return (
        <Box component="span" sx={{display: "inline-flex", flexDirection: "column"}}>
            <Button
                size="small"
                variant="text"
                startIcon={<DownloadIcon fontSize="inherit" />}
                disabled={loading}
                onClick={() => void download()}
                aria-label={t("dashboard.ballotBoxes.downloadRecord", {area: areaName})}
                sx={{justifyContent: "flex-start", px: 0.5, minWidth: 0}}
            >
                {t("dashboard.ballotBoxes.column.record")}
            </Button>
            {failed ? (
                <Typography variant="caption" color="error">
                    {failed === ERecordDownloadFailure.MISSING
                        ? t("dashboard.ballotBoxes.recordMissing")
                        : t("dashboard.ballotBoxes.recordError")}
                </Typography>
            ) : null}
        </Box>
    )
}

export default SealRecordLink
