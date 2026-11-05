// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"
import {
    Alert,
    Box,
    Button,
    Chip,
    CircularProgress,
    FormControl,
    InputLabel,
    Link,
    MenuItem,
    Select,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import DownloadIcon from "@mui/icons-material/Download"
import {downloadUrl} from "@sequentech/ui-core"
import {requestStatusKey} from "@/lib/signing/status"
import {SigningRequestStatus, type ISigningRequestListRow} from "@/lib/signing/types"
import {DownloadDocument} from "@/resources/User/DownloadDocument"
import {useSigningRequest} from "@/components/signing/SigningProvider"
import {filterRequests, lastSignature, personName, requestStatus} from "./signingSettings"
import {
    useExportRequests,
    useScopeNames,
    useSigningRequests,
    useWriteError,
} from "./useSigningSettings"
import type {ISignaturesSubTabProps} from "./ProtectedActionsTab"

const ALL = "all"

// A waiting request gets a warning border: warning-colored text is too faint to read.
const STATUS_COLORS: Record<SigningRequestStatus, "success" | "error" | "default"> = {
    [SigningRequestStatus.Waiting]: "default",
    [SigningRequestStatus.Completed]: "success",
    [SigningRequestStatus.Executed]: "success",
    [SigningRequestStatus.Cancelled]: "default",
    [SigningRequestStatus.Expired]: "default",
    [SigningRequestStatus.Failed]: "error",
}

/** Every signing request of the event; each opens the signing request panel, where it can be cancelled. */
export const RequestsTab: React.FC<ISignaturesSubTabProps> = ({electionEventId, access}) => {
    const {t} = useTranslation()
    const writeError = useWriteError()
    const panel = useSigningRequest()
    const {requests, loading, error} = useSigningRequests(electionEventId)
    const {postName, countryName} = useScopeNames(electionEventId, true)
    const [status, setStatus] = useState<SigningRequestStatus | null>(null)
    const [exported, setExported] = useState<string | null>(null)
    const [exportRequests, {loading: exporting}] = useExportRequests()
    const time = useEventZonedFormat(electionEventId).format

    const runExport = async () => {
        try {
            const {data} = await exportRequests({
                variables: {election_event_id: electionEventId},
            })
            const result = data?.signingExportRequests
            // The export's own link needs no document permission; without one, the
            // document routes download it.
            if (result?.url) {
                await downloadUrl(result.url, t("signing.requests.exportFileName"))
                return
            }
            if (!result?.document_id) throw new Error("no document")
            setExported(result.document_id)
        } catch (error) {
            writeError(error, "signing.requests.exportError")
        }
    }

    if (error) {
        return <Alert severity="error">{t("signing.loadError")}</Alert>
    }
    if (loading || !requests) {
        return <CircularProgress aria-label={t("common.label.loadingData")} />
    }

    const now = new Date()
    const shown = filterRequests(requests, status, now)
    const title = (request: ISigningRequestListRow) =>
        [
            t(`signing.actions.${request.action}.short`),
            request.election_id ? postName(request.election_id) : null,
            request.area_id ? countryName(request.area_id) : null,
        ]
            .filter(Boolean)
            .join(" · ")

    const statusLabel = (request: ISigningRequestListRow) => {
        const name = t(requestStatusKey(requestStatus(request, now)))
        return request.status === SigningRequestStatus.Cancelled
            ? name
            : t("signing.requests.statusCount", {
                  status: name,
                  count: request.approvals.length,
                  total: request.required,
              })
    }

    return (
        <Box>
            <Box sx={{display: "flex", alignItems: "center", gap: 2, mb: 2}}>
                <FormControl size="small" sx={{minWidth: 240}}>
                    <InputLabel id="signing-request-status">
                        {t("signing.requests.status")}
                    </InputLabel>
                    <Select
                        labelId="signing-request-status"
                        label={t("signing.requests.status")}
                        value={status ?? ALL}
                        onChange={(event) =>
                            setStatus(
                                event.target.value === ALL
                                    ? null
                                    : (event.target.value as SigningRequestStatus)
                            )
                        }
                    >
                        <MenuItem value={ALL}>{t("signing.requests.statusAll")}</MenuItem>
                        {Object.values(SigningRequestStatus).map((value) => (
                            <MenuItem key={value} value={value}>
                                {t(requestStatusKey(value))}
                            </MenuItem>
                        ))}
                    </Select>
                </FormControl>
                <Box sx={{flex: 1}} />
                {!access.requestsCancel && (
                    <Chip size="small" variant="outlined" label={t("signing.readOnly.chip")} />
                )}
                {access.requestsExport && (
                    <Button
                        variant="outlined"
                        startIcon={<DownloadIcon />}
                        disabled={exporting || !!exported}
                        onClick={runExport}
                    >
                        {t("signing.requests.exportCsv")}
                    </Button>
                )}
            </Box>
            <TableContainer sx={{border: 1, borderColor: "divider", borderRadius: 1}}>
                <Table size="small" aria-label={t("signing.tab.requests")}>
                    <TableHead>
                        <TableRow>
                            <TableCell>{t("signing.requests.columns.request")}</TableCell>
                            <TableCell>{t("signing.requests.columns.status")}</TableCell>
                            <TableCell>{t("signing.requests.columns.started")}</TableCell>
                            <TableCell>{t("signing.requests.columns.by")}</TableCell>
                            <TableCell>{t("signing.requests.columns.lastSignature")}</TableCell>
                            <TableCell>{t("signing.requests.columns.code")}</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {shown.length === 0 && (
                            <TableRow>
                                <TableCell colSpan={6}>{t("signing.requests.empty")}</TableCell>
                            </TableRow>
                        )}
                        {shown.map((request) => {
                            const last = lastSignature(request)
                            const shownStatus = requestStatus(request, now)
                            return (
                                <TableRow
                                    key={request.id}
                                    hover
                                    sx={{cursor: "pointer"}}
                                    onClick={() => panel.open(request.id)}
                                >
                                    <TableCell>
                                        <Link
                                            component="button"
                                            underline="hover"
                                            color="inherit"
                                            sx={{fontWeight: 600, textAlign: "left"}}
                                            onClick={(event: React.MouseEvent) => {
                                                event.stopPropagation()
                                                panel.open(request.id)
                                            }}
                                        >
                                            {title(request)}
                                        </Link>
                                        <Typography variant="body2" color="text.secondary">
                                            {t(`signing.actions.${request.action}.label`)}
                                        </Typography>
                                    </TableCell>
                                    <TableCell>
                                        <Chip
                                            size="small"
                                            variant={
                                                shownStatus === SigningRequestStatus.Waiting
                                                    ? "outlined"
                                                    : "filled"
                                            }
                                            color={STATUS_COLORS[shownStatus]}
                                            sx={
                                                shownStatus === SigningRequestStatus.Waiting
                                                    ? {borderColor: "warning.main"}
                                                    : undefined
                                            }
                                            label={statusLabel(request)}
                                        />
                                        <Typography variant="body2" color="text.secondary">
                                            {shownStatus === SigningRequestStatus.Waiting &&
                                            request.expires_at
                                                ? t("signing.requests.expires", {
                                                      time: time(request.expires_at),
                                                  })
                                                : request.status ===
                                                        SigningRequestStatus.Cancelled &&
                                                    request.cancel_reason
                                                  ? t(
                                                        `signing.cancelReasons.${request.cancel_reason}`
                                                    )
                                                  : null}
                                        </Typography>
                                    </TableCell>
                                    <TableCell>{time(request.created_at)}</TableCell>
                                    <TableCell>
                                        {personName(
                                            request.requested_by_name,
                                            request.requested_by_username
                                        )}
                                    </TableCell>
                                    <TableCell>
                                        {last
                                            ? t("signing.requests.lastSignatureBy", {
                                                  name: personName(
                                                      last.display_name,
                                                      last.username
                                                  ),
                                                  time: time(last.signed_at),
                                              })
                                            : "–"}
                                    </TableCell>
                                    <TableCell sx={{fontFamily: "monospace"}}>
                                        {request.code}
                                    </TableCell>
                                </TableRow>
                            )
                        })}
                    </TableBody>
                </Table>
            </TableContainer>
            {exported && (
                <DownloadDocument
                    documentId={exported}
                    electionEventId={electionEventId}
                    fileName={t("signing.requests.exportFileName")}
                    onDownload={() => setExported(null)}
                />
            )}
        </Box>
    )
}
