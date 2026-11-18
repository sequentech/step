// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Box} from "@mui/system"
import {isRouteErrorResponse, Link, useRouteError} from "react-router-dom"
import {useTranslation} from "react-i18next"
import {Button, Typography} from "@mui/material"
import {Header, HeaderErrorVariant} from "@sequentech/ui-essentials"
import {stringToHtml} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {AuthContext} from "../providers/AuthContextProvider"
import {useAppSelector} from "../store/hooks"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {selectKnownElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {
    EventStyles,
    eventLogo,
    eventStylesClassName,
    eventCss,
} from "../components/EventBranding/EventBranding"
import {usePageBranding} from "@sequentech/ui-core"

const StyledLink = styled(Link)`
    text-decoration: none;
    margin-top: 40px;
`

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 36px;
    justify-content: center;
`

export function ErrorPage() {
    const error = useRouteError()
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const {globalSettings} = useContext(SettingsContext)
    const authContext = useContext(AuthContext)
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const electionEvent = useAppSelector(selectKnownElectionEvent)
    const presentation =
        ballotStyle?.ballot_eml.election_event_presentation ?? electionEvent?.presentation
    const customCss = eventCss(presentation)
    usePageBranding(presentation)

    const isErrorType = error instanceof Error || error instanceof VotingPortalError

    let content = (
        <>
            <StyledTitle
                className="screen-title"
                variant="h3"
                component="h1"
                sx={{marginBottom: "24px"}}
            >
                {t("errors.page.oopsWithoutStatus")}
            </StyledTitle>
            <Typography
                className="error-description"
                variant="h6"
                component="div"
                sx={{marginBottom: "24px"}}
            >
                {stringToHtml(t("errors.page.somethingWrong"))}
            </Typography>
        </>
    )

    if (isRouteErrorResponse(error)) {
        content = (
            <>
                <StyledTitle
                    className="screen-title"
                    variant="h3"
                    component="h1"
                    sx={{marginBottom: "24px"}}
                >
                    {t("errors.page.oopsWithStatus", {status: error.status})}
                </StyledTitle>
                <Typography
                    className="error-description"
                    variant="h6"
                    component="p"
                    sx={{marginBottom: "24px"}}
                >
                    {error.statusText}
                </Typography>
                {error.data?.message && (
                    <Typography className="error-description">
                        <i className="error-detail">{error.data.message}</i>
                    </Typography>
                )}
            </>
        )
    } else if (
        error instanceof VotingPortalError &&
        error.type === VotingPortalErrorType.CERT_AUTH_FAILED
    ) {
        content = (
            <>
                <StyledTitle
                    className="screen-title"
                    variant="h3"
                    component="h1"
                    sx={{marginBottom: "24px"}}
                >
                    {t("errors.page.certAuthFailedTitle")}
                </StyledTitle>
                <Typography
                    className="error-description"
                    variant="h6"
                    component="div"
                    sx={{marginBottom: "24px"}}
                >
                    {stringToHtml(t("errors.page.certAuthFailedMessage"))}
                </Typography>
            </>
        )
    } else if (
        error instanceof VotingPortalError &&
        error.type === VotingPortalErrorType.INVALID_LOGIN_HINT_PARAMETERS
    ) {
        content = (
            <>
                <StyledTitle
                    className="screen-title"
                    variant="h3"
                    component="h1"
                    sx={{marginBottom: "24px"}}
                >
                    {t("errors.page.invalidLoginHintParametersTitle")}
                </StyledTitle>
                <Typography
                    className="error-description"
                    variant="h6"
                    component="div"
                    sx={{marginBottom: "24px"}}
                >
                    {stringToHtml(t("errors.page.invalidLoginHintParametersMessage"))}
                </Typography>
            </>
        )
    } else if (isErrorType) {
        content = (
            <>
                <StyledTitle
                    className="screen-title"
                    variant="h3"
                    component="h1"
                    sx={{marginBottom: "24px"}}
                >
                    {t("errors.page.oopsWithoutStatus")}
                </StyledTitle>
                <Typography
                    className="error-description"
                    variant="h6"
                    component="div"
                    sx={{marginBottom: "24px"}}
                >
                    {stringToHtml(t("errors.page.somethingWrong"))}
                </Typography>
                <Typography className="error-description">
                    <i className="error-detail">{error.message}</i>
                </Typography>
            </>
        )
    }

    return (
        <EventStyles className={eventStylesClassName(presentation)} customCss={customCss}>
            <Box sx={{minHeight: "100vh"}} className="voting-portal app-root error-screen screen">
                <Header
                    appVersion={{main: globalSettings.APP_VERSION}}
                    logoUrl={eventLogo(presentation)}
                    errorVariant={HeaderErrorVariant.HIDE_PROFILE}
                    logoutFn={authContext.isAuthenticated ? authContext.logout : undefined}
                />
                <Box
                    className="error-content"
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
                        <StyledLink className="error-back-link" to={backLink}>
                            <Button className="back-button" sx={{textDecoration: "none"}}>
                                {t("common.goBack")}
                            </Button>
                        </StyledLink>
                    )}
                </Box>
            </Box>
        </EventStyles>
    )
}
