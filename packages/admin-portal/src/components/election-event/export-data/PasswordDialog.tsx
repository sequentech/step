// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {IconButton, TextField, Tooltip, Typography} from "@mui/material"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"

export const DecryptHelp: React.FC<{decryptionCommand: string}> = ({decryptionCommand}) => {
    const notify = useNotify()
    const {t} = useTranslation()

    const handleCopyPassword = () => {
        navigator.clipboard
            .writeText(decryptionCommand)
            .then(() => {
                notify(t("electionEventScreen.export.copiedSuccess"), {
                    type: "success",
                })
            })
            .catch((err) => {
                notify(t("electionEventScreen.export.copiedError"), {
                    type: "error",
                })
            })
    }

    return (
        <>
            <Typography sx={{whiteSpace: "pre-wrap"}}>
                {t("reportsScreen.messages.decryptInstructions")}
            </Typography>
            <TextField
                fullWidth
                value={decryptionCommand}
                multiline={true}
                InputProps={{
                    readOnly: true,
                    endAdornment: (
                        <Tooltip
                            title={t("electionEventScreen.import.passwordDialog.copyPassword")}
                        >
                            <IconButton onClick={handleCopyPassword}>
                                <ContentCopyIcon />
                            </IconButton>
                        </Tooltip>
                    ),
                }}
            />
        </>
    )
}

export interface PasswordDialogProps {
    password: string
    onClose: () => void
    children?: React.ReactNode
}

export const PasswordDialog: React.FC<PasswordDialogProps> = ({password, onClose, children}) => {
    const {t} = useTranslation()
    const notify = useNotify()

    const handleCopyPassword = () => {
        navigator.clipboard
            .writeText(password)
            .then(() => {
                notify(t("electionEventScreen.export.copiedSuccess"), {
                    type: "success",
                })
            })
            .catch((err) => {
                notify(t("electionEventScreen.export.copiedError"), {
                    type: "error",
                })
            })
    }

    return (
        <Dialog
            variant="info"
            open={true}
            handleClose={onClose}
            aria-labelledby="password-dialog-title"
            title={t("electionEventScreen.export.passwordTitle")}
            ok={"Ok"}
        >
            {t("electionEventScreen.export.passwordDescription")}
            <TextField
                fullWidth
                margin="normal"
                value={password}
                InputProps={{
                    readOnly: true,
                    endAdornment: (
                        <Tooltip
                            title={t("electionEventScreen.import.passwordDialog.copyPassword")}
                        >
                            <IconButton onClick={handleCopyPassword}>
                                <ContentCopyIcon />
                            </IconButton>
                        </Tooltip>
                    ),
                }}
            />
            {children}
        </Dialog>
    )
}
