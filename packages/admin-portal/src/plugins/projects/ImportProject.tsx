// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useRef, useState} from "react"
import {
    Alert,
    Autocomplete,
    Box,
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
    TextField,
    Typography,
} from "@mui/material"
import moment from "moment-timezone"
import {useTranslation} from "react-i18next"
import {PluginHost, errorMessage} from "../contracts"
import {ImportPreview, MAX_TITLE, Operation, Project, Saved} from "./model"

export enum NewProjectMode {
    Create = "create",
    Import = "import",
}

const MAX_CSV_BYTES = 1048576
const CSV_COLUMNS = "id,title,type,start,end,status,notes"

interface Props {
    mode: NewProjectMode
    host: PluginHost
    onClose: () => void
    onCreated: (project: Saved<Project>) => void
}

export const ImportProject: React.FC<Props> = ({mode, host, onClose, onCreated}) => {
    const {t} = useTranslation()
    const zones = useMemo(() => moment.tz.names(), [])
    const [name, setName] = useState("")
    const [timezone, setTimezone] = useState<string | null>(
        moment.tz.zone(Intl.DateTimeFormat().resolvedOptions().timeZone)
            ? Intl.DateTimeFormat().resolvedOptions().timeZone
            : "UTC"
    )
    const [fileName, setFileName] = useState("")
    const [csv, setCsv] = useState("")
    const [preview, setPreview] = useState<ImportPreview | null>(null)
    const [busy, setBusy] = useState(false)
    const [error, setError] = useState("")
    const fileInput = useRef<HTMLInputElement>(null)
    const importing = mode === NewProjectMode.Import
    const nameValid = name.trim().length > 0 && Array.from(name.trim()).length <= MAX_TITLE
    const ready = nameValid && Boolean(timezone) && (!importing || Boolean(preview?.valid))
    const invalidRows = preview?.rows.filter((row) => row.errors.length).length ?? 0

    const read = async (file?: File) => {
        setPreview(null)
        setCsv("")
        setError("")
        setFileName(file?.name ?? "")
        if (!file) return
        if (file.size > MAX_CSV_BYTES) {
            setError(t("projects.import.fileLimit"))
            return
        }
        setBusy(true)
        try {
            const text = new TextDecoder("utf-8", {fatal: true}).decode(await file.arrayBuffer())
            setCsv(text)
            setPreview(await host.execute<ImportPreview>(Operation.PreviewImport, {csv: text}))
        } catch (cause) {
            setError(
                cause instanceof TypeError ? t("projects.import.invalidFile") : errorMessage(cause)
            )
        } finally {
            setBusy(false)
        }
    }

    const create = async () => {
        if (!ready || !timezone) return
        setBusy(true)
        setError("")
        try {
            onCreated(
                await host.execute<Saved<Project>>(
                    importing ? Operation.CommitImport : Operation.CreateProject,
                    importing ? {name: name.trim(), timezone, csv} : {name: name.trim(), timezone}
                )
            )
        } catch (cause) {
            setError(errorMessage(cause))
            setBusy(false)
        }
    }

    return (
        <Dialog
            open
            onClose={() => !busy && onClose()}
            maxWidth={importing ? "md" : "sm"}
            fullWidth
        >
            <DialogTitle>
                {t(importing ? "projects.import.title" : "projects.create.title")}
            </DialogTitle>
            <DialogContent>
                <Stack spacing={2.5} sx={{mt: 1}}>
                    {error && <Alert severity="error">{error}</Alert>}
                    <TextField
                        label={t("projects.create.name")}
                        value={name}
                        onChange={(event) => setName(event.target.value)}
                        disabled={busy}
                        inputProps={{maxLength: MAX_TITLE}}
                        required
                        autoFocus
                    />
                    <Autocomplete
                        options={zones}
                        value={timezone}
                        onChange={(_, value) => setTimezone(value)}
                        disabled={busy}
                        renderInput={(params) => (
                            <TextField
                                {...params}
                                label={t("projects.create.timezone")}
                                helperText={t("projects.create.timezoneHelp")}
                                required
                            />
                        )}
                    />
                    {importing && (
                        <>
                            <Alert severity="info">
                                {t("projects.import.help", {columns: CSV_COLUMNS})}
                            </Alert>
                            <Stack direction="row" spacing={2} alignItems="center">
                                <Button
                                    variant="outlined"
                                    disabled={busy}
                                    onClick={() => fileInput.current?.click()}
                                >
                                    {t("projects.import.choose")}
                                </Button>
                                <input
                                    ref={fileInput}
                                    type="file"
                                    accept=".csv,text/csv"
                                    hidden
                                    aria-label={t("projects.import.choose")}
                                    onChange={(event) => void read(event.target.files?.[0])}
                                />
                                <Typography variant="body2" color="text.secondary" noWrap>
                                    {fileName || t("projects.import.noFile")}
                                </Typography>
                            </Stack>
                            {preview && (
                                <>
                                    <Alert
                                        severity={preview.valid ? "success" : "error"}
                                        role="status"
                                    >
                                        {preview.valid
                                            ? t("projects.import.valid", {
                                                  count: preview.rows.length,
                                              })
                                            : t("projects.import.invalid", {
                                                  count: invalidRows,
                                                  total: preview.rows.length,
                                              })}
                                    </Alert>
                                    <Box sx={{overflowX: "auto"}}>
                                        <Table
                                            size="small"
                                            aria-label={t("projects.import.previewTable")}
                                        >
                                            <TableHead>
                                                <TableRow>
                                                    <TableCell>
                                                        {t("projects.import.line")}
                                                    </TableCell>
                                                    <TableCell>
                                                        {t("projects.editor.title")}
                                                    </TableCell>
                                                    <TableCell>
                                                        {t("projects.import.result")}
                                                    </TableCell>
                                                </TableRow>
                                            </TableHead>
                                            <TableBody>
                                                {preview.rows.map((row) => (
                                                    <TableRow
                                                        key={row.line}
                                                        sx={{
                                                            bgcolor: row.errors.length
                                                                ? "error.50"
                                                                : undefined,
                                                        }}
                                                    >
                                                        <TableCell>{row.line}</TableCell>
                                                        <TableCell>
                                                            {row.task?.title || "—"}
                                                        </TableCell>
                                                        <TableCell
                                                            sx={{
                                                                color: row.errors.length
                                                                    ? "error.main"
                                                                    : "success.main",
                                                            }}
                                                        >
                                                            {row.errors.join("; ") ||
                                                                t("projects.import.rowValid")}
                                                        </TableCell>
                                                    </TableRow>
                                                ))}
                                            </TableBody>
                                        </Table>
                                    </Box>
                                </>
                            )}
                        </>
                    )}
                </Stack>
            </DialogContent>
            <DialogActions>
                <Button disabled={busy} onClick={onClose}>
                    {t("projects.cancel")}
                </Button>
                <Button variant="contained" disabled={busy || !ready} onClick={() => void create()}>
                    {t(importing ? "projects.import.commit" : "projects.create.submit")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
