// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useState} from "react"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    BreadCrumbSteps,
    Icon,
    IconButton,
    theme,
    stringToHtml,
    isUndefined,
    Dialog,
    translateElection,
} from "@sequentech/ui-essentials"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {faCircleQuestion, faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {Link as RouterLink, redirect, useNavigate, useParams, useSubmit} from "react-router-dom"
import {
    selectBallotSelectionByElectionId,
    setBallotSelection,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {provideBallotService} from "../services/BallotService"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {CircularProgress} from "@mui/material"
import {selectElectionById} from "../store/elections/electionsSlice"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
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

interface ActionButtonProps {
    disableNext: boolean
    handleNext: () => void
}

const ActionButtons: React.FC<ActionButtonProps> = ({handleNext, disableNext}) => {
    const {t} = useTranslation()
    const backLink = useRootBackLink()

    return (
        <ActionsContainer>
            <StyledLink to={backLink} sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                    <Icon icon={faAngleLeft} size="sm" />
                    <Box>{t("votingScreen.backButton")}</Box>
                </StyledButton>
            </StyledLink>
            <StyledButton
                sx={{width: {xs: "100%", sm: "200px"}}}
                onClick={() => handleNext()}
                disabled={disableNext}
            >
                <Box>{t("votingScreen.reviewButton")}</Box>
                <Icon icon={faAngleRight} size="sm" />
            </StyledButton>
        </ActionsContainer>
    )
}

const VotingScreen: React.FC = () => {
    const {t, i18n} = useTranslation()

    const {electionId} = useParams<{electionId?: string}>()
    const {tenantId, eventId} = useParams<TenantEventType>()

    let [disableNext, setDisableNext] = useState<Record<string, boolean>>({})
    const [openBallotHelp, setOpenBallotHelp] = useState(false)

    const {encryptBallotSelection, decodeAuditableBallot} = provideBallotService()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle?.election_id ?? "")
    )

    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const dispatch = useAppDispatch()

    const submit = useSubmit()

    const onSetDisableNext = (id: string) => (value: boolean) => {
        setDisableNext({
            ...disableNext,
            [id]: value,
        })
    }

    const skipNextButton = Object.values(disableNext).some((v) => v)

    const encryptAndReview = () => {
        if (isUndefined(selectionState) || skipNextButton || !ballotStyle) {
            return
        }

        try {
            const startMs = Date.now()
            const auditableBallot = encryptBallotSelection(selectionState, ballotStyle.ballot_eml)
            const endMs = Date.now()
            console.log(`Success encrypting ballot: ${endMs - startMs} ms`, auditableBallot)

            dispatch(
                setAuditableBallot({
                    electionId: ballotStyle?.election_id ?? "",
                    auditableBallot,
                })
            )

            let decodedSelectionState = decodeAuditableBallot(auditableBallot)

            if (decodedSelectionState !== null) {
                dispatch(
                    setBallotSelection({
                        ballotStyle,
                        ballotSelection: decodedSelectionState,
                    })
                )
            }

            submit(null)
        } catch (error) {
            submit({error: "Unable to encrypt the Ballot"}, {method: "post"})
        }
    }

    useEffect(() => {
        if (!election || !ballotStyle) {
            navigate(backLink)
        }
    }, [navigate, backLink, election, ballotStyle])

    if (!ballotStyle || !election) {
        return <CircularProgress />
    }

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="48px">
                <BreadCrumbSteps
                    labels={[
                        "breadcrumbSteps.electionList",
                        "breadcrumbSteps.ballot",
                        "breadcrumbSteps.review",
                        "breadcrumbSteps.confirmation",
                    ]}
                    selected={1}
                />
            </Box>
            <StyledTitle variant="h4">
                <Box>{translateElection(election, "name", i18n.language) || ""}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setOpenBallotHelp(true)}
                />
                <Dialog
                    handleClose={() => setOpenBallotHelp(false)}
                    open={openBallotHelp}
                    title={t("votingScreen.ballotHelpDialog.title")}
                    ok={t("votingScreen.ballotHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("votingScreen.ballotHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            {election.description ? (
                <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                    {stringToHtml(translateElection(election, "description", i18n.language))}
                </Typography>
            ) : null}
            {ballotStyle.ballot_eml.contests.map((question, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={question}
                    questionIndex={index}
                    key={index}
                    isReview={false}
                    setDisableNext={onSetDisableNext(question.id)}
                />
            ))}
            <ActionButtons handleNext={encryptAndReview} disableNext={skipNextButton} />
        </PageLimit>
    )
}

export default VotingScreen

export async function action({request}: {request: Request}) {
    const data = await request.formData()
    const error = data.get("error")

    if (error) {
        throw new Error(error as string)
    }

    redirect(`review`)
}
