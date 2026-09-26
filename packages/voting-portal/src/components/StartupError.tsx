// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, Button} from "@mui/material"
import {useTranslation} from "react-i18next"

export const StartupError = () => {
    const {t} = useTranslation()
    return (
        <Alert
            className="startup-error"
            severity="error"
            action={
                <Button className="startup-retry" onClick={() => window.location.reload()}>
                    {t("startup.retry")}
                </Button>
            }
        >
            {t("startup.error")}
        </Alert>
    )
}
