// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Dialog} from "@sequentech/ui-essentials"
import {EBoothRefusalState} from "../../services/vitrine/boothCheck"
import {vitrineLanguage} from "../../services/vitrine/language"
import type {IBoothRefusal} from "../../hooks/useBoothCheck"
import {BOARD, EGlyph, Glyph, quietTextSx} from "./board"
import {VitrineStatusView} from "./VitrineElements"

export interface IBoothRefusalDialogProps {
    refusal?: IBoothRefusal
    onRetry: () => void
    onClose: () => void
}

/** Why the booth didn't encrypt the ballot (step-integration.md, section
 * 2.6): the design board Alert's mark, the kit's status and the portal's own
 * calm explanation. */
export const BoothRefusalDialog: React.FC<IBoothRefusalDialogProps> = ({
    refusal,
    onRetry,
    onClose,
}) => {
    const {t, i18n} = useTranslation()
    const state = refusal?.state ?? EBoothRefusalState.PENDING
    const message = t(`vitrine.booth.${state}`)
    const waiting = state === EBoothRefusalState.PENDING
    const color = waiting ? BOARD.subtle : BOARD.warning
    return (
        <Dialog
            className="vitrine-booth-refusal-dialog"
            handleClose={(retry) => (retry ? onRetry() : onClose())}
            open={Boolean(refusal)}
            title={t("vitrine.booth.title")}
            ok={t("vitrine.booth.retry")}
            cancel={t("vitrine.booth.close")}
            variant={waiting ? "info" : "softwarning"}
        >
            <Box
                className="vitrine-booth-refusal"
                data-reason={refusal?.reason}
                sx={{display: "flex", flexDirection: "column", gap: "12px"}}
            >
                <Box
                    className="vitrine-booth-icon"
                    aria-hidden="true"
                    sx={{
                        width: 48,
                        height: 48,
                        borderRadius: "50%",
                        border: `2.5px solid ${color}`,
                        bgcolor: waiting ? BOARD.tint : BOARD.warningTint,
                        color,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                    }}
                >
                    <Glyph
                        glyph={waiting ? EGlyph.REFRESH : EGlyph.EXCLAIM}
                        size={waiting ? 24 : 28}
                        stroke={waiting ? 2.4 : 3.2}
                    />
                </Box>
                <Typography
                    className="vitrine-booth-explanation"
                    variant="body2"
                    component="p"
                    sx={{...quietTextSx, margin: 0, fontSize: "14px", color: BOARD.body}}
                >
                    {t("vitrine.booth.explanation")}
                </Typography>
                <VitrineStatusView
                    className="vitrine-booth-status"
                    state={state}
                    message={message}
                    lang={vitrineLanguage(i18n.language)}
                    fallback={
                        <Typography
                            className="vitrine-booth-status-text"
                            variant="body2"
                            component="p"
                        >
                            {message}
                        </Typography>
                    }
                />
            </Box>
        </Dialog>
    )
}
