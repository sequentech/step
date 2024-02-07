// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    Dialog,
    Icon,
    IconButton,
    PageLimit,
    isUndefined,
    stringToHtml,
    theme,
    translateElection,
} from "@sequentech/ui-essentials"
import React, {useContext, useEffect, useState} from "react"
import {Link as RouterLink, redirect, useNavigate, useParams, useSubmit} from "react-router-dom"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import {faAngleLeft, faAngleRight, faCircleQuestion} from "@fortawesome/free-solid-svg-icons"
import {
    resetBallotSelection,
    selectBallotSelectionByElectionId,
    setBallotSelection,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"

import {AuthContext} from "../providers/AuthContextProvider"
import {Box} from "@mui/material"
import Button from "@mui/material/Button"
import {CircularProgress} from "@mui/material"
import {Question} from "../components/Question/Question"
import Stepper from "../components/Stepper"
import Typography from "@mui/material/Typography"
import {canVoteSomeElection} from "../store/castVotes/castVotesSlice"
import {provideBallotService} from "../services/BallotService"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionById} from "../store/elections/electionsSlice"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {sortContestByCreationDate} from "../lib/utils"
import {styled} from "@mui/material/styles"
import {useRootBackLink} from "../hooks/root-back-link"
import {useTranslation} from "react-i18next"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
`

const StyledTitle = styled(Typography)`
    margin-top: 25.5px;
    display: flex;
    flex-direction: row;
    gap: 16px;
    font-size: 36px;
    justify-content: center;
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
    handleNext: () => void
}

const ActionButtons: React.FC<ActionButtonProps> = ({handleNext}) => {
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()

    function handleClearSelection() {
        if (ballotStyle) {
            dispatch(
                resetBallotSelection({
                    ballotStyle,
                    force: true,
                })
            )
        }
    }

    return (
        <>
            <StyledButton
                sx={{
                    display: {sm: "none"},
                    width: "100%",
                }}
                variant="secondary"
                onClick={() => handleClearSelection()}
            >
                <Box>{t("votingScreen.clearButton")}</Box>
            </StyledButton>

            <ActionsContainer>
                <StyledLink to={backLink} sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}>
                    <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box>{t("votingScreen.backButton")}</Box>
                    </StyledButton>
                </StyledLink>

                <StyledButton
                    sx={{
                        display: {xs: "none", sm: "block"},
                        width: {xs: "100%", sm: "200px"},
                    }}
                    variant="secondary"
                    onClick={() => handleClearSelection()}
                >
                    <Box>{t("votingScreen.clearButton")}</Box>
                </StyledButton>

                <StyledButton
                    className="next-button"
                    sx={{width: {xs: "100%", sm: "200px"}}}
                    onClick={() => handleNext()}
                    // disabled={disableNext}
                >
                    <Box>{t("votingScreen.reviewButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </>
    )
}

const VotingScreen: React.FC = () => {
    const {t, i18n} = useTranslation()
    const {logout} = useContext(AuthContext)
    const canVote = useAppSelector(canVoteSomeElection())

    const {electionId} = useParams<{electionId?: string}>()

    let [disableNext, setDisableNext] = useState<Record<string, boolean>>({})
    const [openBallotHelp, setOpenBallotHelp] = useState(false)
    const [openNotVoted, setOpenNonVoted] = useState(false)

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

    // if true, when the user clicks next, there will be a dialog
    // that doesn't allow to continue and forces the user to fix the issues
    const skipNextButton = Object.values(disableNext).some((v) => v)

    const encryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        } else if (skipNextButton) {
            setOpenNonVoted(true)
        } else {
            finallyEncryptAndReview()
        }
    }

    const finallyEncryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        }
        try {
            const startMs = Date.now()
            const auditableBallot = encryptBallotSelection(selectionState, ballotStyle.ballot_eml)
            const endMs = Date.now()

            dispatch(
                setAuditableBallot({
                    electionId: ballotStyle?.election_id ?? "",
                    auditableBallot,
                })
            )

            let decodedSelectionState = decodeAuditableBallot(auditableBallot)

            if (null !== decodedSelectionState) {
                dispatch(
                    setBallotSelection({
                        ballotStyle,
                        ballotSelection: decodedSelectionState,
                    })
                )
            }

            submit(null, {method: "post"})
        } catch (error) {
            submit({error: VotingPortalErrorType.UNABLE_TO_CAST_BALLOT}, {method: "post"})
        }
    }

    useEffect(() => {
        if (!election || !ballotStyle) {
            navigate(backLink)
        } else if (!selectionState || !canVote) {
            logout()
        }
    }, [navigate, backLink, election, ballotStyle, selectionState, canVote, logout])

    useEffect(() => {
        let minMaxGlobal = false
        for (let contest of ballotStyle?.ballot_eml.contests ?? []) {
            let votes = 0
            let selection = selectionState?.find((s) => s.contest_id === contest.id)
            for (let choice of selection?.choices ?? []) {
                if (choice.selected > -1) {
                    votes = choice.selected + 1
                }
            }
            let outOfRange = votes < contest.min_votes || votes > contest.max_votes
            minMaxGlobal = minMaxGlobal || outOfRange
        }
        setDisableNext({
            ...disableNext,
            minmax_global: minMaxGlobal,
        })
    }, [selectionState, ballotStyle])

    if (!ballotStyle || !election) {
        return <CircularProgress />
    }

    const contests = sortContestByCreationDate(ballotStyle.ballot_eml.contests)

    return (
        <PageLimit maxWidth="lg">
            <Box marginTop="48px">
                <Stepper selected={1} />
            </Box>
            <StyledTitle variant="h4">
                <Box className="selected-election-title">
                    {translateElection(election, "name", i18n.language) ?? "-"}
                </Box>
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
                    {stringToHtml(translateElection(election, "description", i18n.language) ?? "-")}
                </Typography>
            ) : null}
            {contests.map((contest, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={contest}
                    questionIndex={contest.originalIndex}
                    key={index}
                    isReview={false}
                    setDisableNext={onSetDisableNext(contest.id)}
                    isUniqChecked={true} // TODO: make it configurable
                />
            ))}
            <ActionButtons handleNext={encryptAndReview} />

            <Dialog
                handleClose={() => setOpenNonVoted(false)}
                open={openNotVoted}
                title={t("votingScreen.nonVotedDialog.title")}
                ok={t("votingScreen.nonVotedDialog.ok")}
                variant="softwarning"
            >
                {stringToHtml(t("votingScreen.nonVotedDialog.content"))}
            </Dialog>
        </PageLimit>
    )
}

export default VotingScreen

export async function action({request}: {request: Request}) {
    const data = await request.formData()
    const error = data.get("error")

    if (error) {
        throw new VotingPortalError(
            VotingPortalErrorType[error as keyof typeof VotingPortalErrorType]
        )
    }

    return redirect(`../review`)
}
