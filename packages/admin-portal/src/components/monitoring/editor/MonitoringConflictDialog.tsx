// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Box,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
} from "@mui/material"
import {DiffView} from "@/components/DiffView"
import type {IMonitoringAuthor} from "./types"

export enum EClipboardResult {
    NONE = "NONE",
    COPIED = "COPIED",
    FAILED = "FAILED",
}

export interface MonitoringConflictDialogProps {
    open: boolean
    /** The YAML in the editor, which the save carried. */
    mine: string
    /** The YAML of the revision saved meanwhile; `undefined` while it loads. */
    theirs?: string
    currentRevision: number
    author?: IMonitoringAuthor | null
    time?: string | null
    /** Replaces the draft with the saved revision. */
    onReload: () => void
    /** Keeps the draft; the next save replaces the revision shown here. */
    onKeepEditing: () => void
    /** Where "Copy my YAML" writes; the browser clipboard by default. */
    copyText?: (text: string) => Promise<void>
}

/** DiffView compares JSON; a list of lines keeps the YAML as written, comments included. */
const asLines = (text: string) => text.split("\n")

const clipboardWrite = (text: string) => {
    if (typeof navigator === "undefined" || !navigator.clipboard) {
        return Promise.reject(new Error("no clipboard"))
    }
    return navigator.clipboard.writeText(text)
}

const noMorePages = async () => undefined

/** Someone saved the document while it was being edited: compare, then choose. */
export const MonitoringConflictDialog: React.FC<MonitoringConflictDialogProps> = ({
    open,
    mine,
    theirs,
    currentRevision,
    author,
    time,
    onReload,
    onKeepEditing,
    copyText = clipboardWrite,
}) => {
    const {t, i18n} = useTranslation()
    const [clipboard, setClipboard] = useState(EClipboardResult.NONE)
    const date = time
        ? new Intl.DateTimeFormat(i18n.language, {dateStyle: "medium", timeStyle: "short"}).format(
              new Date(time)
          )
        : null
    const user = author?.name || author?.id
    const copy = async () => {
        try {
            await copyText(mine)
            setClipboard(EClipboardResult.COPIED)
        } catch {
            setClipboard(EClipboardResult.FAILED)
        }
    }

    return (
        <Dialog
            open={open}
            onClose={onKeepEditing}
            maxWidth="lg"
            fullWidth
            aria-labelledby="monitoring-conflict-title"
        >
            <DialogTitle id="monitoring-conflict-title">
                {t("monitoring.editor.conflict.title")}
            </DialogTitle>
            <DialogContent sx={{display: "flex", flexDirection: "column", gap: 2}}>
                <DialogContentText>
                    {user && date
                        ? t("monitoring.editor.conflict.body", {
                              user,
                              revision: currentRevision,
                              date,
                          })
                        : t("monitoring.editor.conflict.bodyShort", {revision: currentRevision})}
                </DialogContentText>
                {clipboard === EClipboardResult.COPIED ? (
                    <Alert severity="success">{t("monitoring.editor.conflict.copied")}</Alert>
                ) : null}
                {clipboard === EClipboardResult.FAILED ? (
                    <Alert severity="warning">{t("monitoring.editor.conflict.copyFailed")}</Alert>
                ) : null}
                {theirs === undefined ? (
                    <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                        <CircularProgress aria-label={t("monitoring.editor.conflict.saved")} />
                    </Box>
                ) : (
                    <DiffView
                        currentTitle={t("monitoring.editor.conflict.saved")}
                        diffTitle={t("monitoring.editor.conflict.mine")}
                        current={asLines(theirs)}
                        modify={asLines(mine)}
                        fetchAllPublishChanges={noMorePages}
                    />
                )}
            </DialogContent>
            <DialogActions>
                <Button onClick={copy}>{t("monitoring.editor.conflict.copy")}</Button>
                <Button onClick={onReload} disabled={theirs === undefined}>
                    {t("monitoring.editor.conflict.reload")}
                </Button>
                <Button variant="contained" onClick={onKeepEditing}>
                    {t("monitoring.editor.conflict.keepEditing")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

export default MonitoringConflictDialog
