// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import {
    Button,
    Dialog,
    DialogActions,
    DialogContent,
    DialogContentText,
    DialogTitle,
} from "@mui/material"

export interface MonitoringDiscardDialogProps {
    open: boolean
    /** What is lost: "Your changes to this widget have not been saved." */
    body: string
    onKeepEditing: () => void
    onDiscard: () => void
}

/** Asked before an editor with unsaved changes closes, however it was asked to close. */
export const MonitoringDiscardDialog: React.FC<MonitoringDiscardDialogProps> = ({
    open,
    body,
    onKeepEditing,
    onDiscard,
}) => {
    const {t} = useTranslation()
    return (
        <Dialog open={open} onClose={onKeepEditing} aria-labelledby="monitoring-discard-title">
            <DialogTitle id="monitoring-discard-title">
                {t("monitoring.editor.configureWidget.discardTitle")}
            </DialogTitle>
            <DialogContent>
                <DialogContentText>{body}</DialogContentText>
            </DialogContent>
            <DialogActions>
                <Button onClick={onKeepEditing}>
                    {t("monitoring.editor.configureWidget.keepEditing")}
                </Button>
                <Button color="error" onClick={onDiscard}>
                    {t("monitoring.editor.configureWidget.discard")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}

export default MonitoringDiscardDialog
