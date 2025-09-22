// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Box, Checkbox, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Dialog, PageLimit, theme} from "@sequentech/ui-essentials"
import {
    IElection,
    stringToHtml,
    translateElection,
    EStartScreenTitlePolicy,
    ESecurityConfirmationPolicy,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import {Link as RouterLink, useLocation, useNavigate, useParams} from "react-router-dom"
import Button from "@mui/material/Button"
import {useAppSelector} from "../store/hooks"
import {selectElectionById} from "../store/elections/electionsSlice"
import {CircularProgress} from "@mui/material"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import Stepper from "../components/Stepper"
import {selectBallotStyleByElectionId, showDemo} from "../store/ballotStyles/ballotStylesSlice"
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
    display: flex;
    padding: 5px;

    span {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        padding: 5px;
    }
`

const StyledCheckboxWrapper = styled(Box)`
    display: flex;
    flex-direction: row;
    cursor: pointer;
    align-items: flex-start;
    padding: 10px 0;
`

const StyledCheckbox = styled(Checkbox)`
    margin-top: 4px;
    margin-right: 9px;
    padding: 0;
`
interface ActionButtonsProps {
    election: IElection
}

const ActionButtons: React.FC<ActionButtonsProps> = ({election}) => {
    const {t, i18n} = useTranslation()
    const {tenantId, eventId} = useParams<TenantEventType>()
    const location = useLocation()
    const [checkboxChecked, setCheckboxChecked] = useState(false)

    const hasSecurityCheckbox =
        ESecurityConfirmationPolicy.MANDATORY ===
        election?.presentation?.security_confirmation_policy
    const defaultTranslation = translateElection(election, "security_confirmation_html", "en")
    const disabledStart = hasSecurityCheckbox && !checkboxChecked

    return (
        <>
            {hasSecurityCheckbox ? (
                <StyledCheckboxWrapper onClick={() => setCheckboxChecked(!checkboxChecked)}>
                    <StyledCheckbox checked={checkboxChecked} />
                    <Typography variant="body2" marginTop="4px">
                        {stringToHtml(
                            translateElection(
                                election,
                                "security_confirmation_html",
                                i18n.language
                            ) ??
                                defaultTranslation ??
                                "-"
                        )}
                    </Typography>
                </StyledCheckboxWrapper>
            ) : null}
            <ActionsContainer>
                {disabledStart ? (
                    <StyledButton
                        className="start-voting-button"
                        sx={{width: "100%"}}
                        disabled={true}
                    >
                        {t("startScreen.startButton")}
                    </StyledButton>
                ) : (
                    <StyledLink
                        to={`/tenant/${tenantId}/event/${eventId}/election/${election.id}/vote${location.search}`}
                        sx={{margin: "auto 0", width: "100%"}}
                    >
                        <StyledButton className="start-voting-button" sx={{width: "100%"}}>
                            {t("startScreen.startButton")}
                        </StyledButton>
                    </StyledLink>
                )}
            </ActionsContainer>
        </>
    )
}

const StartScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const {electionId} = useParams<{electionId?: string}>()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const {eventId, tenantId} = useParams<{eventId?: string; tenantId?: string}>()
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const backLink = useRootBackLink()
    const isDemo = useAppSelector(showDemo(electionId))
    const [showDemoDialog, setShowDemoDialog] = useState(isDemo)
    const navigate = useNavigate()
    useLanguage({ballotStyle})

    const titleObject = useMemo(() => {
        const startScreenTitlePolicy = election?.presentation?.start_screen_title_policy
        return startScreenTitlePolicy === EStartScreenTitlePolicy.ELECTION_EVENT
            ? electionEvent
            : election
    }, [election, electionEvent])

    useEffect(() => {
        if (!election || !titleObject) {
            navigate(backLink)
        }
    })

    if (!election || !titleObject) {
        return <CircularProgress />
    }

    return (
        <PageLimit maxWidth="lg" className="start-screen screen">
            <Box marginTop="48px">
                <Stepper selected={1} />
            </Box>
            <StyledTitle variant="h3" justifyContent="center" fontWeight="bold">
                <span>{translateElection(titleObject, "name", i18n.language) ?? "-"}</span>
            </StyledTitle>
            {titleObject.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(
                        translateElection(titleObject, "description", i18n.language) ?? "-"
                    )}
                </Typography>
            ) : null}
            <Typography variant="h5">{t("startScreen.instructionsTitle")}</Typography>
            <Typography variant="body2">{t("startScreen.instructionsDescription")}</Typography>
            <Box
                sx={{
                    display: "flex",
                    flexDirection: {xs: "column", md: "row"},
                    gap: {sm: 0, md: "15px"},
                }}
            >
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {t("startScreen.step1Title")}
                    </Typography>
                    <Typography variant="body2">{t("startScreen.step1Description")}</Typography>
                </Box>
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {t("startScreen.step2Title")}
                    </Typography>
                    <Typography variant="body2">{t("startScreen.step2Description")}</Typography>
                </Box>
                <Box sx={{width: {xs: "100%", md: "33.33333333%"}}}>
                    <Typography variant="h5" sx={{color: theme.palette.brandColor}}>
                        {t("startScreen.step3Title")}
                    </Typography>
                    <Typography variant="body2">{t("startScreen.step3Description")}</Typography>
                </Box>
            </Box>
            <ActionButtons election={election} />

            <Dialog
                variant="warning"
                open={showDemoDialog}
                ok={t("electionSelectionScreen.demoDialog.ok")}
                title={t("electionSelectionScreen.demoDialog.title")}
                handleClose={(result: boolean) => {
                    setShowDemoDialog(false)
                }}
                fullWidth
                className="demo-dialog"
            >
                {stringToHtml(t("electionSelectionScreen.demoDialog.content"))}
            </Dialog>
        </PageLimit>
    )
}

export default StartScreen
