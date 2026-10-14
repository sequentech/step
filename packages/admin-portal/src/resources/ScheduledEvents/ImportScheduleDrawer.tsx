// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {OutcomeChanges} from "./OutcomeChangeNotice"
import React, {useId, useRef, useState} from "react"
import {useNotify} from "react-admin"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Drawer,
    Link,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {useApolloClient, useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import type {GetUploadUrlMutation} from "@/gql/graphql"
import {GET_UPLOAD_URL} from "@/queries/GetUploadUrl"
import {
    IMPORT_SCHEDULE,
    PREVIEW_SCHEDULE_IMPORT,
    type ImportScheduleData,
    type PreviewScheduleImportData,
} from "@/queries/Lifecycle"
import type {IScheduleImportPreview, IScheduleImportRow} from "@/types/lifecycle"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {formatWallTime, useTimeZoneService} from "@/components/timezones/timeZoneService"

/** The CSV columns (design §6); the template has the header and two example rows. */
export const SCHEDULE_CSV_COLUMNS = [
    "election_alias",
    "event_type",
    "local_date_time",
    "timezone",
    "voting_channels",
] as const

const TEMPLATE_ROWS = [
    ["ALL", "END_VOTING_PERIOD", "2028-05-08T19:00", "", ""],
    ["<election alias>", "START_VOTING_PERIOD", "2028-04-09T00:00", "", "ONLINE|KIOSK"],
]

/** Rows shown before "…and N more rows". */
const SHOWN_ROWS = 7

/** The error codes `preview_schedule_import` reports per row (stream C). */
const ERROR_KEYS: Record<string, string> = {
    "unknown-election": "unknownElection",
    "ambiguous-election": "ambiguousElection",
    "unknown-event-type": "unknownEventType",
    "invalid-time-zone": "invalidTimeZone",
    "invalid-date-time": "invalidDateTime",
    "invalid-voting-channels": "invalidVotingChannels",
    "dst-gap": "dstGap",
    "duplicate": "duplicate",
}

/** Errors first, then in file order. */
export const previewOrder = (rows: ReadonlyArray<IScheduleImportRow>): Array<IScheduleImportRow> =>
    [...rows].sort((a, b) => Number(!a.error_code) - Number(!b.error_code) || a.row - b.row)

const templateUrl = () => {
    const csv = [SCHEDULE_CSV_COLUMNS.join(","), ...TEMPLATE_ROWS.map((row) => row.join(","))]
    return URL.createObjectURL(new Blob([csv.join("\n") + "\n"], {type: "text/csv"}))
}

/** One preview row: its time in its zone and in the primary, or what's wrong with it. */
const PreviewRow: React.FC<{row: IScheduleImportRow; primary: string}> = ({row, primary}) => {
    const {t} = useTranslation()
    const service = useTimeZoneService()
    // Spreadsheets may write a space for the T.
    const local = row.local.trim().replace(" ", "T").slice(0, 16)
    const error = row.error_code
    // A time in a DST gap doesn't exist: the import refuses it (design §1 defaults).
    const errorText = error
        ? t(`lifecycle.import.error.${ERROR_KEYS[error] ?? "other"}`, {
              code: error,
              zone: row.time_zone,
              election: row.election_alias,
              type: row.event_type,
              dateTime: formatWallTime(local, service.text),
              city: service.zoneCity(row.time_zone, service.text),
          })
        : null
    return (
        <TableRow
            data-testid={`import-row-${row.row}`}
            sx={error ? {bgcolor: "rgba(237, 108, 2, 0.08)"} : undefined}
        >
            <TableCell>{row.row}</TableCell>
            <TableCell>{row.election_name ?? row.election_alias}</TableCell>
            <TableCell>
                {t(`eventsScreen.eventType.${row.event_type}`, {defaultValue: row.event_type})}
            </TableCell>
            <TableCell>
                {row.instant && !error ? (
                    <>
                        <Typography variant="body2">
                            {service.formatDateTimeZone(row.instant, row.time_zone, service.text)}
                        </Typography>
                        {row.time_zone !== primary ? (
                            <Typography variant="body2" color="text.secondary">
                                {service.formatDateTimeZone(row.instant, primary, service.text)}
                            </Typography>
                        ) : null}
                    </>
                ) : (
                    <Typography variant="body2">
                        {t("lifecycle.import.asWritten", {
                            local: row.local,
                            place: row.election_name ?? row.election_alias,
                        })}
                    </Typography>
                )}
                {errorText ? (
                    // The theme's warning colours are too faint on the row's tint (axe color-contrast).
                    <Typography variant="body2" sx={{color: "#8a3a00"}}>
                        {errorText}
                    </Typography>
                ) : row.note_code === "dst-overlap" && !error ? (
                    <Typography variant="body2" color="text.secondary">
                        {service.zonedTimeNote(local, row.time_zone, service.text)}
                    </Typography>
                ) : null}
            </TableCell>
        </TableRow>
    )
}

/**
 * Import schedule: upload a CSV (one row per event and election, in local
 * time), check every row in its zone and in the primary, and import only a
 * file without errors. Importing again updates the same events.
 */
export const ImportScheduleDrawer: React.FC<{
    electionEventId: string
    /** The event's primary zone. */
    primary: string
    onClose: () => void
    onImported: () => void
}> = ({electionEventId, primary, onClose, onImported}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const client = useApolloClient()
    const titleId = useId()
    const fileInput = useRef<HTMLInputElement | null>(null)
    const [file, setFile] = useState<{name: string; documentId: string} | null>(null)
    const [preview, setPreview] = useState<IScheduleImportPreview | null>(null)
    const [busy, setBusy] = useState(false)
    const [getUploadUrl] = useMutation<GetUploadUrlMutation>(GET_UPLOAD_URL)
    const [importSchedule, {loading: importing}] = useMutation<ImportScheduleData>(
        IMPORT_SCHEDULE,
        {context: {headers: {"x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE}}}
    )

    const upload = async (chosen: File) => {
        setBusy(true)
        setPreview(null)
        setFile(null)
        try {
            const {data} = await getUploadUrl({
                variables: {
                    name: chosen.name,
                    media_type: chosen.type || "text/csv",
                    size: chosen.size,
                    is_public: false,
                    election_event_id: electionEventId,
                },
            })
            const target = data?.get_upload_url
            if (!target?.url) throw new Error("No upload URL")
            const response = await fetch(target.url, {
                method: "PUT",
                headers: {"Content-Type": chosen.type || "text/csv"},
                body: chosen,
            })
            if (!response.ok) throw new Error("Upload failed")
            const {data: checked} = await client.mutate<PreviewScheduleImportData>({
                mutation: PREVIEW_SCHEDULE_IMPORT,
                variables: {electionEventId, documentId: target.document_id},
                context: {headers: {"x-hasura-role": IPermissions.SCHEDULED_EVENT_WRITE}},
            })
            if (!checked?.preview_schedule_import) throw new Error("No preview")
            setFile({name: chosen.name, documentId: target.document_id})
            setPreview(checked.preview_schedule_import)
        } catch (error) {
            notify(getGraphQLActionErrorReason(error) ?? t("lifecycle.import.uploadError"), {
                type: "error",
            })
        } finally {
            setBusy(false)
        }
    }

    const onImport = async () => {
        if (!file) return
        try {
            const {data} = await importSchedule({
                variables: {electionEventId, documentId: file.documentId},
            })
            notify(
                t("lifecycle.import.imported", {
                    created: data?.import_schedule?.created ?? 0,
                    updated: data?.import_schedule?.updated ?? 0,
                }),
                {type: "success"}
            )
            onImported()
        } catch (error) {
            notify(getGraphQLActionErrorReason(error) ?? t("lifecycle.import.importError"), {
                type: "error",
            })
        }
    }

    const rows = preview ? previewOrder(preview.rows) : []
    const clean = !!preview && preview.errors === 0

    return (
        <Drawer
            anchor="right"
            open
            onClose={onClose}
            slotProps={{paper: {"aria-labelledby": titleId}}}
            sx={{"& .MuiDrawer-paper": {width: {xs: "100%", md: 900}, maxWidth: "100%"}}}
        >
            <Stack spacing={2} sx={{width: "100%", boxSizing: "border-box", p: 3}}>
                <Box>
                    <Typography id={titleId} variant="h5" component="h2">
                        {t("lifecycle.import.title")}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        {t("lifecycle.import.subtitle")}
                    </Typography>
                </Box>
                <Stack direction="row" spacing={2} sx={{alignItems: "center"}}>
                    <Button
                        variant="contained"
                        disabled={busy || importing}
                        onClick={() => fileInput.current?.click()}
                        startIcon={busy ? <CircularProgress size={14} aria-hidden /> : undefined}
                    >
                        {file?.name ?? t("lifecycle.import.chooseFile")}
                    </Button>
                    <input
                        ref={fileInput}
                        type="file"
                        accept=".csv,text/csv"
                        hidden
                        data-testid="schedule-file"
                        onChange={(event) => {
                            const chosen = event.target.files?.[0]
                            event.target.value = ""
                            if (chosen) upload(chosen).catch(() => undefined)
                        }}
                    />
                    <Link
                        component="button"
                        type="button"
                        onClick={() => {
                            const url = templateUrl()
                            const anchor = document.createElement("a")
                            anchor.href = url
                            anchor.download = t("lifecycle.import.templateFileName")
                            anchor.click()
                            URL.revokeObjectURL(url)
                        }}
                    >
                        {t("lifecycle.import.template")}
                    </Link>
                </Stack>
                {preview ? (
                    <Alert severity={clean ? "success" : "warning"} data-testid="import-summary">
                        {clean
                            ? t("lifecycle.import.ready", {ok: preview.ok, posts: preview.posts})
                            : t("lifecycle.import.needsAttention", {
                                  ok: preview.ok,
                                  posts: preview.posts,
                                  count: preview.errors,
                              })}
                    </Alert>
                ) : null}
                {preview ? (
                    <>
                        <OutcomeChanges
                            changes={preview.outcome_changes ?? []}
                            zone={preview.primary_time_zone ?? primary}
                        />
                        <Table size="small" aria-label={t("lifecycle.import.preview")}>
                            <TableHead>
                                <TableRow>
                                    <TableCell>{t("lifecycle.import.row")}</TableCell>
                                    <TableCell>{t("eventsScreen.fields.electionId")}</TableCell>
                                    <TableCell>{t("eventsScreen.fields.eventProcessor")}</TableCell>
                                    <TableCell>{t("eventsScreen.fields.scheduledDate")}</TableCell>
                                </TableRow>
                            </TableHead>
                            <TableBody>
                                {rows.slice(0, SHOWN_ROWS).map((row) => (
                                    <PreviewRow
                                        key={row.row}
                                        row={row}
                                        primary={preview.primary_time_zone ?? primary}
                                    />
                                ))}
                            </TableBody>
                        </Table>
                        {rows.length > SHOWN_ROWS ? (
                            <Typography variant="body2" color="text.secondary">
                                {t("lifecycle.import.moreRows", {count: rows.length - SHOWN_ROWS})}
                            </Typography>
                        ) : null}
                    </>
                ) : null}
                <Stack direction="row" spacing={1} sx={{justifyContent: "flex-end"}}>
                    <Button onClick={onClose}>{t("common.label.cancel")}</Button>
                    <Button variant="contained" disabled={!clean || importing} onClick={onImport}>
                        {t("common.label.import")}
                    </Button>
                </Stack>
            </Stack>
        </Drawer>
    )
}
