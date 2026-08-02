// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {
    Alert,
    Button,
    IconButton,
    InputAdornment,
    Stack,
    TextField,
    Typography,
} from "@mui/material"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"

interface VoterInformationLetterPasswordAccessProps {
    pdfPassword?: string
    loading?: boolean
    onReveal?: () => void
}

export const VoterInformationLetterPasswordAccess = ({
    pdfPassword,
    loading = false,
    onReveal,
}: VoterInformationLetterPasswordAccessProps) => {
    const {t} = useTranslation()
    const notify = useNotify()

    const copyPassword = async () => {
        if (!pdfPassword) {
            return
        }
        try {
            await navigator.clipboard.writeText(pdfPassword)
            notify(t("tasksScreen.documentAccess.passwordCopied"), {type: "success"})
        } catch {
            notify(t("tasksScreen.documentAccess.copyError"), {type: "error"})
        }
    }

    return (
        <Stack spacing={2} alignItems="flex-start">
            <Alert severity="warning" sx={{width: "100%"}}>
                {t("tasksScreen.documentAccess.sensitivityNotice")}
            </Alert>
            <Typography variant="body2" fontWeight={500}>
                {t("tasksScreen.documentAccess.passwordLabel")}:
            </Typography>
            {!pdfPassword ? (
                <Button
                    variant="outlined"
                    startIcon={<VisibilityIcon />}
                    onClick={onReveal}
                    disabled={loading || !onReveal}
                    sx={(theme) => ({
                        "alignSelf": "flex-start",
                        "backgroundColor": theme.palette.white,
                        "borderColor": theme.palette.brandColor,
                        "color": theme.palette.brandColor,
                        "&:hover": {
                            backgroundColor: theme.palette.brandColor,
                            borderColor: theme.palette.brandColor,
                            color: theme.palette.white,
                        },
                    })}
                >
                    {t("tasksScreen.documentAccess.showPassword")}
                </Button>
            ) : null}
            <Typography variant="body2" color="text.secondary">
                {t("tasksScreen.documentAccess.guidance")}
            </Typography>
            {pdfPassword ? (
                <TextField
                    fullWidth
                    value={pdfPassword}
                    sx={{
                        "maxWidth": 420,
                        "alignSelf": "flex-start",
                        "& input": {fontFamily: "monospace"},
                    }}
                    slotProps={{
                        htmlInput: {
                            "aria-label": t("tasksScreen.documentAccess.passwordLabel"),
                        },
                        input: {
                            readOnly: true,
                            endAdornment: (
                                <InputAdornment position="end">
                                    <IconButton
                                        onClick={() => void copyPassword()}
                                        edge="end"
                                        aria-label={t("tasksScreen.documentAccess.copyPassword")}
                                    >
                                        <ContentCopyIcon />
                                    </IconButton>
                                </InputAdornment>
                            ),
                        },
                    }}
                />
            ) : null}
        </Stack>
    )
}
