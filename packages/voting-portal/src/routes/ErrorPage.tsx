// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Box} from "@mui/system"
import {isRouteErrorResponse, Link, useParams, useRouteError} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Button, Typography} from "@mui/material"
import {Header, HeaderErrorVariant, translateText} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {TenantEventType} from ".."
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {useAppSelector} from "../store/hooks"

const StyledLink = styled(Link)`
    text-decoration: none;
    margin-top: 40px;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 36px;
    justify-content: center;
`

export function ErrorPage() {
    const error = useRouteError()
    const {t, i18n} = useTranslation()
    const {eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const backLink = useRootBackLink()
    const {globalSettings} = useContext(SettingsContext)
    const authContext = useContext(AuthContext)

    const isErrorType = error instanceof Error || error instanceof VotingPortalError

    let content = (
        <>
            <StyledTitle variant="h3" sx={{marginBottom: "24px"}}>
                {translateText(
                    electionEvent,
                    "errors.page.oopsWithoutStatus",
                    i18n.language,
                    t("errors.page.oopsWithoutStatus")
                )}
            </StyledTitle>
            <Typography variant="h6" sx={{marginBottom: "24px"}}>
                {translateText(
                    electionEvent,
                    "errors.page.somethingWrong",
                    i18n.language,
                    t("errors.page.somethingWrong")
                )}
            </Typography>
        </>
    )

    if (isRouteErrorResponse(error)) {
        content = (
            <>
                <StyledTitle variant="h3" sx={{marginBottom: "24px"}}>
                    {translateText(
                        electionEvent,
                        "errors.page.oopsWithStatus",
                        i18n.language,
                        t("errors.page.oopsWithStatus", {status: error.status})
                    )}
                </StyledTitle>
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
                <StyledTitle variant="h3" sx={{marginBottom: "24px"}}>
                    {translateText(
                        electionEvent,
                        "errors.page.oopsWithoutStatus",
                        i18n.language,
                        t("errors.page.oopsWithoutStatus")
                    )}
                </StyledTitle>
                <Typography variant="h6" sx={{marginBottom: "24px"}}>
                    {translateText(
                        electionEvent,
                        "errors.page.somethingWrong",
                        i18n.language,
                        t("errors.page.somethingWrong")
                    )}
                </Typography>
                <Typography>
                    <i>{error.message}</i>
                </Typography>
            </>
        )
    }

    return (
        <Box sx={{minHeight: "100vh"}} className="error-screen screen">
            <Header
                appVersion={{main: globalSettings.APP_VERSION}}
                errorVariant={HeaderErrorVariant.HIDE_PROFILE}
                logoutFn={authContext.logout}
            />
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
                        <Button sx={{textDecoration: "none"}}>
                            {translateText(
                                electionEvent,
                                "common.goBack",
                                i18n.language,
                                t("common.goBack")
                            )}
                        </Button>
                    </StyledLink>
                )}
            </Box>
        </Box>
    )
}
