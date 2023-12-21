// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React from "react"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"

export const NotFoundScreen: React.FC = () => {
    const {t} = useTranslation()
    return (
        <Box
            sx={{
                width: "100%",
                height: "50vh",
                display: "flex",
                flexDirection: "column",
                justifyContent: "center",
                alignItems: "center",
            }}
        >
            <Typography variant="h3" sx={{marginBottom: "24px"}}>
                {t("404.title")}
            </Typography>
            <Typography variant="h6">{t("404.subtitle")}</Typography>
        </Box>
    )
}
