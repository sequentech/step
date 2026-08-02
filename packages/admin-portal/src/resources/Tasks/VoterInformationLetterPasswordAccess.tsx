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
    className?: string
}

export const VoterInformationLetterPasswordAccess = ({
    pdfPassword,
    loading = false,
    onReveal,
    className,
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
        <Stack
            className={["voter-information-letter-password-access", className]
                .filter(Boolean)
                .join(" ")}
            spacing={2}
            alignItems="flex-start"
        >
            <Alert
                className="voter-information-letter-password-access-warning"
                severity="warning"
                sx={{width: "100%"}}
            >
                {t("tasksScreen.documentAccess.sensitivityNotice")}
            </Alert>
            <Typography
                className="voter-information-letter-password-access-label"
                variant="body2"
                fontWeight={500}
            >
                {t("tasksScreen.documentAccess.passwordLabel")}:
            </Typography>
            {!pdfPassword ? (
                <Button
                    className="voter-information-letter-password-access-show-button"
                    variant="outlined"
                    startIcon={
                        <VisibilityIcon className="voter-information-letter-password-access-show-icon" />
                    }
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
            <Typography
                className="voter-information-letter-password-access-guidance"
                variant="body2"
                color="text.secondary"
            >
                {t("tasksScreen.documentAccess.guidance")}
            </Typography>
            {pdfPassword ? (
                <TextField
                    className="voter-information-letter-password-access-field"
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
                            "className": "voter-information-letter-password-access-input",
                        },
                        input: {
                            className: "voter-information-letter-password-access-input-root",
                            readOnly: true,
                            endAdornment: (
                                <InputAdornment
                                    className="voter-information-letter-password-access-copy-adornment"
                                    position="end"
                                >
                                    <IconButton
                                        className="voter-information-letter-password-access-copy-button"
                                        onClick={() => void copyPassword()}
                                        edge="end"
                                        aria-label={t("tasksScreen.documentAccess.copyPassword")}
                                    >
                                        <ContentCopyIcon className="voter-information-letter-password-access-copy-icon" />
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
