// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Alert, AlertTitle, List, ListItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {ISlateProblem} from "@sequentech/ui-core"

interface SlatesWarningsProps {
    warnings: Array<ISlateProblem>
}

export const SlatesWarnings: React.FC<SlatesWarningsProps> = ({warnings}) => {
    const {t} = useTranslation()
    if (warnings.length === 0) {
        return null
    }
    return (
        <Alert severity="warning" sx={{marginBottom: "1rem"}}>
            <AlertTitle>{t("electionScreen.slates.warnings")}</AlertTitle>
            <List dense disablePadding>
                {warnings.map((warning) => (
                    <ListItem
                        key={`${warning.code}:${warning.path}:${warning.message}`}
                        disableGutters
                    >
                        {`${warning.path}: ${warning.message}`}
                    </ListItem>
                ))}
            </List>
        </Alert>
    )
}
