// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useRef, useState} from "react"
import {gql, useMutation, useQuery} from "@apollo/client"
import {Alert, Box, Button, TextField, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {downloadUrl} from "@sequentech/ui-core"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ETallyType} from "@/types/ceremonies"
import {loadBrowserTrustee} from "@/services/BrowserTrustee"
import {
    ProtocolBoardRecord,
    VerificationBoard,
    verificationBoards,
} from "@/services/verificationExport"

const VERIFICATION_BOARDS = gql`
    query VerificationBoards($tenantId: uuid!, $eventId: uuid!) {
        sequent_backend_protocol_board(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $eventId}}
        ) {
            id
            tenant_id
            election_event_id
            parent_id
            keys_ceremony_id
            tally_session_id
            batch
            name
            manager_message
        }
    }
`
const EXPORT_VERIFICATION_TRANSCRIPT = gql`
    mutation ExportVerificationTranscript($eventId: String!, $boardId: String!) {
        export_verification_transcript(election_event_id: $eventId, protocol_board_id: $boardId) {
            document_id
            url
        }
    }
`
const context = {headers: {"x-hasura-role": IPermissions.ADMIN_CEREMONY}}
interface ExportResponse {
    export_verification_transcript?: {document_id: string; url: string} | null
}
export interface VerificationExportProps {
    eventId: string
    tallySessionId: string
    completed: boolean
    tallyType: string | null | undefined
}

/** Public export and independently sourced anchors; no trustee keys or board-service access. */
export function VerificationExport({
    eventId,
    tallySessionId,
    completed,
    tallyType,
}: VerificationExportProps) {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const auth = useContext(AuthContext)
    const enabled =
        completed &&
        tallyType === ETallyType.ELECTORAL_RESULTS &&
        !!tenantId &&
        auth.isAuthorized(true, tenantId, IPermissions.ADMIN_CEREMONY)
    const scope = JSON.stringify([tenantId, eventId, tallySessionId, enabled])
    const activeScope = useRef(scope)
    activeScope.current = scope
    const {data, loading, error} = useQuery<{
        sequent_backend_protocol_board: ProtocolBoardRecord[]
    }>(VERIFICATION_BOARDS, {
        variables: {tenantId, eventId},
        context,
        skip: !enabled,
        fetchPolicy: "network-only",
    })
    const [exportTranscript] = useMutation<ExportResponse>(EXPORT_VERIFICATION_TRANSCRIPT)
    const [prepared, setPrepared] = useState<{scope: string; boards: VerificationBoard[]}>()
    const [failedScope, setFailedScope] = useState<string>()
    const [downloadError, setDownloadError] = useState<string>()
    const [busy, setBusy] = useState<string>()
    useEffect(() => {
        setBusy(undefined)
        setDownloadError(undefined)
    }, [scope])
    useEffect(() => {
        let cancelled = false
        if (!enabled || !tenantId || !data || loading || error) return
        void loadBrowserTrustee()
            .then((module) => {
                const boards = verificationBoards(
                    data.sequent_backend_protocol_board,
                    {tenantId, eventId, tallySessionId},
                    module.WasmVerifier
                )
                if (!cancelled) {
                    setPrepared({scope, boards})
                    setFailedScope(undefined)
                }
            })
            .catch(() => {
                if (!cancelled) setFailedScope(scope)
            })
        return () => {
            cancelled = true
        }
    }, [data, loading, error, enabled, tenantId, eventId, tallySessionId, scope])
    useEffect(
        () => () => {
            activeScope.current = ""
        },
        []
    )
    if (!enabled) return null
    if (error || failedScope === scope)
        return (
            <Alert severity="error">
                {t("verificationExport.anchorError", {
                    defaultValue: "Verification anchors could not be loaded.",
                })}
            </Alert>
        )
    if (loading || prepared?.scope !== scope) return null
    const download = async (board: VerificationBoard) => {
        setBusy(board.id)
        setDownloadError(undefined)
        try {
            const {data: exported, errors} = await exportTranscript({
                variables: {eventId, boardId: board.id},
                context,
            })
            const result = exported?.export_verification_transcript
            if (errors?.length || !result?.document_id || !result.url)
                throw new Error("Export failed")
            const url = new URL(result.url)
            if (url.protocol !== "https:" || url.username || url.password)
                throw new Error("Invalid download URL")
            if (activeScope.current !== scope) return
            await downloadUrl(result.url, `verification-${board.id}.json`)
        } catch {
            if (activeScope.current === scope) setDownloadError(scope)
        } finally {
            if (activeScope.current === scope) setBusy(undefined)
        }
    }
    return (
        <Box sx={{my: 2}}>
            <Typography variant="h6">
                {t("verificationExport.title", {defaultValue: "Election verification"})}
            </Typography>
            <Typography>
                {t("verificationExport.anchors", {
                    defaultValue:
                        "Record these anchors from the election platform separately from the downloaded verification data. Downloading the data does not verify it.",
                })}
            </Typography>
            {prepared.boards.length === 0 && (
                <Typography>
                    {t("verificationExport.empty", {
                        defaultValue: "No verification boards are available for this tally.",
                    })}
                </Typography>
            )}
            {prepared.boards.map((board) => (
                <Box key={board.id} sx={{my: 2}} data-verification-board={board.id}>
                    <Typography>{board.name}</Typography>
                    <TextField
                        fullWidth
                        margin="dense"
                        label={t("verificationExport.configurationHash", {
                            defaultValue: "Configuration hash",
                        })}
                        value={board.configurationHash}
                        InputProps={{readOnly: true}}
                    />
                    <TextField
                        fullWidth
                        margin="dense"
                        label={t("verificationExport.tallyId", {defaultValue: "Tally ID"})}
                        value={board.tallyId}
                        InputProps={{readOnly: true}}
                    />
                    <Button disabled={!!busy} onClick={() => void download(board)}>
                        {t("verificationExport.download", {
                            defaultValue: "Download verification data",
                        })}
                    </Button>
                </Box>
            ))}
            {downloadError === scope && (
                <Alert severity="error">
                    {t("verificationExport.downloadError", {
                        defaultValue: "Verification data could not be downloaded.",
                    })}
                </Alert>
            )}
        </Box>
    )
}
