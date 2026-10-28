// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Dialog, theme} from "@sequentech/ui-essentials"
import {EBoothRefusalState} from "../../services/vitrine/boothCheck"
import {vitrineLanguage} from "../../services/vitrine/language"
import type {IBoothRefusal} from "../../hooks/useBoothCheck"
import {VitrineStatusView} from "./VitrineElements"

export interface IBoothRefusalDialogProps {
    refusal?: IBoothRefusal
    onRetry: () => void
    onClose: () => void
}

/** Why the booth didn't encrypt the ballot (step-integration.md, section
 * 2.6): the kit's status and the portal's own calm explanation. */
export const BoothRefusalDialog: React.FC<IBoothRefusalDialogProps> = ({
    refusal,
    onRetry,
    onClose,
}) => {
    const {t, i18n} = useTranslation()
    const state = refusal?.state ?? EBoothRefusalState.PENDING
    const message = t(`vitrine.booth.${state}`)
    return (
        <Dialog
            className="vitrine-booth-refusal-dialog"
            handleClose={(retry) => (retry ? onRetry() : onClose())}
            open={Boolean(refusal)}
            title={t("vitrine.booth.title")}
            ok={t("vitrine.booth.retry")}
            cancel={t("vitrine.booth.close")}
            variant={state === EBoothRefusalState.PENDING ? "info" : "softwarning"}
        >
            <Box className="vitrine-booth-refusal" data-reason={refusal?.reason}>
                <Typography
                    className="vitrine-booth-explanation"
                    variant="body2"
                    component="p"
                    sx={{color: theme.palette.customGrey.main, marginBottom: "12px"}}
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
