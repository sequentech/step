// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box} from "@mui/system"
import {isRouteErrorResponse, Link, useRouteError} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Button, Typography} from "@mui/material"
import {Header} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"

const StyledLink = styled(Link)`
    text-decoration: none;
    margin-top: 40px;
`

export function ErrorPage() {
    const error = useRouteError()
    const {t} = useTranslation()
    const backLink = useRootBackLink()

    const isErrorType = error instanceof Error || error instanceof VotingPortalError

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
    } else if (isErrorType) {
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
        <Box sx={{minHeight: "100vh"}}>
            <Header />
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

                {!(
                    error instanceof VotingPortalError &&
                    error.type === VotingPortalErrorType.NO_ELECTION_EVENT
                ) && (
                    <StyledLink to={backLink}>
                        <Button sx={{textDecoration: "none"}}>{t("common.goBack")}</Button>
                    </StyledLink>
                )}
            </Box>
        </Box>
    )
}
