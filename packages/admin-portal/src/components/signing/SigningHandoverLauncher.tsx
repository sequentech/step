// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useId, useState} from "react"
import {useTranslation} from "react-i18next"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    Typography,
} from "@mui/material"
import LogoutIcon from "@mui/icons-material/Logout"
import {AuthContext} from "@/providers/AuthContextProvider"
import type {ISigningApi, ISigningPanelData} from "@/lib/signing/api"
import {RESUME_KEY, resumeFor, writeResume} from "@/lib/signing/request"
import {useSigningFormat} from "./format"

export interface ISigningHandoverLauncherProps {
    data: ISigningPanelData
    api: ISigningApi
    /** Where the note for the next member is kept; it survives the sign-out. */
    storage?: Storage
}

/**
 * "Next member signs in": records the handover, leaves a note in session
 * storage and signs out, returning to this page. After the next member signs
 * in, SigningProvider reads the note and reopens the request for them.
 */
export const SigningHandoverLauncher: React.FC<ISigningHandoverLauncherProps> = ({
    data,
    api,
    storage,
}) => {
    const {t} = useTranslation()
    const format = useSigningFormat(data.time_zone)
    const {logout} = useContext(AuthContext)
    const titleId = useId()
    const [open, setOpen] = useState(false)
    const [busy, setBusy] = useState(false)
    const [failed, setFailed] = useState(false)
    const expiresAt = data.request.expires_at

    const handOver = async () => {
        setBusy(true)
        setFailed(false)
        try {
            // The note is written first: storage that refuses it stops the handover
            // before it is logged. Signing out returns to this page.
            writeResume(storage ?? window.sessionStorage, resumeFor(data.request))
            await api.handover(data.request.id)
            logout(window.location.href)
        } catch {
            try {
                ;(storage ?? window.sessionStorage).removeItem(RESUME_KEY)
            } catch {
                // Storage is unavailable: nothing was written.
            }
            setFailed(true)
        } finally {
            setBusy(false)
        }
    }

    return (
        <>
            <Button variant="outlined" startIcon={<LogoutIcon />} onClick={() => setOpen(true)}>
                {t("signing.panel.handover")}
            </Button>
            <Dialog open={open} onClose={() => !busy && setOpen(false)} aria-labelledby={titleId}>
                <DialogTitle id={titleId}>{t("signing.widget.handoverDialog.title")}</DialogTitle>
                <DialogContent>
                    <Typography>
                        {expiresAt
                            ? t("signing.dialog.handover", {
                                  time: format.time(expiresAt),
                              })
                            : t("signing.widget.handoverDialog.noExpiry")}
                    </Typography>
                    {failed ? (
                        <Alert severity="error" sx={{mt: 2}}>
                            {t("signing.widget.handoverDialog.error")}
                        </Alert>
                    ) : null}
                </DialogContent>
                <DialogActions>
                    <Button onClick={() => setOpen(false)} disabled={busy}>
                        {t("signing.widget.handoverDialog.back")}
                    </Button>
                    <Button
                        variant="contained"
                        startIcon={
                            busy ? (
                                <CircularProgress size={16} color="inherit" aria-hidden />
                            ) : (
                                <LogoutIcon />
                            )
                        }
                        disabled={busy}
                        onClick={() => {
                            handOver().catch(() => undefined)
                        }}
                    >
                        {t("signing.widget.handoverDialog.confirm")}
                    </Button>
                </DialogActions>
            </Dialog>
        </>
    )
}
