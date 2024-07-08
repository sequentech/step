// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    PageLimit,
    theme,
    stringToHtml,
    translateElection,
    translateText,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import {Link as RouterLink, useLocation, useNavigate, useParams} from "react-router-dom"
import Button from "@mui/material/Button"
import {useAppSelector} from "../store/hooks"
import {IElection, selectElectionById} from "../store/elections/electionsSlice"
import {CircularProgress} from "@mui/material"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import Stepper from "../components/Stepper"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import useLanguage from "../hooks/useLanguage"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"

const StyledTitle = styled(Typography)`
    width: 100%;
    margin-top: 25.5px;
    margin-bottom: 10px;
    display: block;
    box-sizing: border-box;
    font-size: 36px;
    font-weight: 700;
    line-height: 40px;
    word-break: keep-all;
    text-align: center;
    padding-left: 15px;
    padding-right: 15px;
`

const ActionsContainer = styled(Box)`
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    margin-bottom: 20px;
    margin-top: 10px;
    gap: 2px;
`

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const StyledButton = styled(Button)`
    display flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

interface ActionButtonsProps {
    election: IElection
}

const ActionButtons: React.FC<ActionButtonsProps> = ({election}) => {
    const {t, i18n} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const location = useLocation()

    return (
        <ActionsContainer>
            <StyledLink
                to={`/tenant/${tenantId}/event/${eventId}/election/${election.id}/vote${location.search}`}
                sx={{margin: "auto 0", width: "100%"}}
            >
                <StyledButton className="start-voting-button" sx={{width: "100%"}}>
                    {translateText(
                        electionEvent,
                        "startScreen.startButton",
                        i18n.language,
                        t("startScreen.startButton")
                    )}
                </StyledButton>
            </StyledLink>
        </ActionsContainer>
    )
}

const StartScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const {eventId} = useParams<TenantEventType>()
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const backLink = useRootBackLink()
    const navigate = useNavigate()
    useLanguage({ballotStyle})

    useEffect(() => {
        if (!election) {
            navigate(backLink)
        }
    })

    if (!election) {
        return <CircularProgress />
    }

    return (
        <PageLimit maxWidth="lg" className="start-screen screen">
            <Box marginTop="48px">
                <Stepper selected={1} />
            </Box>
            <StyledTitle variant="h3" justifyContent="center" fontWeight="bold">
                <span>{translateElection(election, "name", i18n.language) ?? "-"}</span>
            </StyledTitle>
            {election.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(translateElection(election, "description", i18n.language) ?? "-")}
                </Typography>
            ) : null}
            <Typography variant="h5">
                {translateText(
                    electionEvent,
                    "startScreen.instructionsTitle",
                    i18n.language,
                    t("startScreen.instructionsTitle")
                )}
            </Typography>
            <Typography variant="body2">
                {translateText(
                    electionEvent,
                    "startScreen.instructionsDescription",
                    i18n.language,
                    t("startScreen.instructionsDescription")
                )}
            </Typography>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    gap: {sm: 0, md: "15px"},
                }}
            >
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {translateText(
                            electionEvent,
                            "startScreen.step1Title",
                            i18n.language,
                            t("startScreen.step1Title")
                        )}
                    </Typography>
                    <Typography variant="body2">
                        {translateText(
                            electionEvent,
                            "startScreen.step1Description",
                            i18n.language,
                            t("startScreen.step1Description")
                        )}
                    </Typography>
                </Box>
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {translateText(
                            electionEvent,
                            "startScreen.step2Title",
                            i18n.language,
                            t("startScreen.step2Title")
                        )}
                    </Typography>
                    <Typography variant="body2">
                        {translateText(
                            electionEvent,
                            "startScreen.step2Description",
                            i18n.language,
                            t("startScreen.step2Description")
                        )}
                    </Typography>
                </Box>
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {translateText(
                            electionEvent,
                            "startScreen.step3Title",
                            i18n.language,
                            t("startScreen.step3Title")
                        )}
                    </Typography>
                    <Typography variant="body2">
                        {translateText(
                            electionEvent,
                            "startScreen.step3Description",
                            i18n.language,
                            t("startScreen.step3Description")
                        )}
                    </Typography>
                </Box>
            </Box>
            <ActionButtons election={election} />
        </PageLimit>
    )
}

export default StartScreen
