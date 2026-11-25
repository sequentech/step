// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {Alert, AlertTitle, Typography} from "@mui/material"

import {Sequent_Backend_Tally_Session_Execution} from "../../gql/graphql"
import {ITallyCeremonyStatus} from "@/types/ceremonies"

interface TallyFailureAlertProps {
    tallySessionExecution?: Sequent_Backend_Tally_Session_Execution
}

/**
 * Why a tally session failed: the reason in the viewer's language and the
 * error behind it as the backend reported it. Nothing is shown for a session
 * that didn't fail.
 */
export const TallyFailureAlert: React.FC<TallyFailureAlertProps> = ({tallySessionExecution}) => {
    const {t} = useTranslation()
    const status = tallySessionExecution?.status as ITallyCeremonyStatus | undefined
    const failure = status?.failure
    if (!failure) return null
    return (
        <Alert severity="error" sx={{mb: 2, width: "100%"}} className="tally-failure-alert">
            <AlertTitle>{t("tally.failure.title")}</AlertTitle>
            {t(`tally.failure.reason.${failure.reason}`, {
                defaultValue: t("tally.failure.reason.unknown"),
            })}
            {failure.detail ? (
                <Typography
                    variant="body2"
                    className="tally-failure-detail"
                    sx={{mt: 1, whiteSpace: "pre-wrap", overflowWrap: "anywhere"}}
                >
                    {t("tally.failure.detail")}: {failure.detail}
                </Typography>
            ) : null}
        </Alert>
    )
}
