// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/system"
import {isRouteErrorResponse, useRouteError} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Typography} from "@mui/material"

export function ErrorPage() {
    const error = useRouteError()
    const {t} = useTranslation()

    let content = (
        <>
            <Typography variant="h3" sx={{marginBottom: "24px"}}>
                {t("errors.page.oopsWithoutStatus")}
            </Typography>
            <Typography variant="h6" sx={{marginBottom: "24px"}}>
                {t("errors.page.somethingWrong")}
            </Typography>
        </>
    )

    if (isRouteErrorResponse(error)) {
        content = (
            <>
                <Typography variant="h3" sx={{marginBottom: "24px"}}>
                    {t("errors.page.oopsWithStatus", {status: error.status})}
                </Typography>
                <Typography variant="h6" sx={{marginBottom: "24px"}}>
                    {error.statusText}
                </Typography>
                {error.data?.message && (
                    <Typography>
                        <i>{error.data.message}</i>
                    </Typography>
                )}
            </>
        )
    } else if (error instanceof Error) {
        content = (
            <>
                <Typography variant="h3" sx={{marginBottom: "24px"}}>
                    {t("errors.page.oopsWithoutStatus")}
                </Typography>
                <Typography variant="h6" sx={{marginBottom: "24px"}}>
                    {t("errors.page.somethingWrong")}
                </Typography>
                <Typography>
                    <i>{error.message}</i>
                </Typography>
            </>
        )
    }

    return (
        <>
            <Box
                id="error-page"
                sx={{
                    width: "100%",
                    height: "50vh",
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "center",
                    alignItems: "center",
                }}
            >
                {content}
            </Box>
        </>
    )
}
