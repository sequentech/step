// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    Icon,
    IconButton,
    theme,
    stringToHtml,
    isUndefined,
    Dialog,
    translateElection,
    sortContestList,
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
    resetBallotSelection,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {clearIsVoted, setIsVoted} from "../store/extra/extraSlice"
import {
    check_voting_error_dialog_bool,
    check_voting_not_allowed_next_bool,
    provideBallotService,
} from "../services/BallotService"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {CircularProgress} from "@mui/material"
import {selectElectionById} from "../store/elections/electionsSlice"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import Stepper from "../components/Stepper"
import {AuthContext} from "../providers/AuthContextProvider"
import {canVoteSomeElection} from "../store/castVotes/castVotesSlice"
import {IDecodedVoteContest} from "sequent-core"

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
            dispatch(clearIsVoted())
        }
    }

    function handlePrev() {
        dispatch(clearIsVoted())
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
                <StyledLink 
                    to={backLink} 
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    onClick={() => handlePrev()}
                >
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
    let [decodedContests, setDecodedContests] = useState<Record<string, IDecodedVoteContest>>({})
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
    const onSetDecodedContests = (id: string) => (value: IDecodedVoteContest) => {
        setDecodedContests((prev) => ({
            ...prev,
            [id]: value,
        }))
    }

    // if true, when the user clicks next, there will be a dialog
    // that doesn't allow to continue and forces the user to fix the issues
    const disableNextButton = (): boolean => {
        return check_voting_not_allowed_next_bool(ballotStyle?.ballot_eml.contests, decodedContests)
    }

    const showNextDialog = () => {
        return check_voting_error_dialog_bool(ballotStyle?.ballot_eml.contests, decodedContests)
    }

    const encryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        } else if (showNextDialog() || disableNextButton()) {
            setOpenNonVoted(true)
        } else {
            finallyEncryptAndReview()
        }
        dispatch(setIsVoted(electionId))
    }

    const finallyEncryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        }
        try {
            const auditableBallot = encryptBallotSelection(selectionState, ballotStyle.ballot_eml)

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
            let countVotes = 0
            let selection = selectionState?.find((s) => s.contest_id === contest.id)
            for (let choice of selection?.choices ?? []) {
                if (choice.selected > -1) {
                    countVotes += choice.selected + 1
                }
            }
            let outOfRange = countVotes < contest.min_votes || countVotes > contest.max_votes
            minMaxGlobal = minMaxGlobal || outOfRange
        }

        setDisableNext((state) => ({
            ...state,
            minmax_global: minMaxGlobal,
        }))
    }, [selectionState, ballotStyle])

    if (!ballotStyle || !election) {
        return <CircularProgress />
    }

    const contests = sortContestList(ballotStyle.ballot_eml.contests)

    const warnAllowContinue = (value: boolean) => {
        setOpenNonVoted(false)
        if (value) {
            finallyEncryptAndReview()
        }
    }

    return (
        <PageLimit maxWidth="lg" className="voting-screen screen">
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
                    key={index}
                    isReview={false}
                    setDisableNext={onSetDisableNext(contest.id)}
                    setDecodedContests={onSetDecodedContests(contest.id)}
                />
            ))}
            <ActionButtons handleNext={encryptAndReview} />

            {disableNextButton() ? (
                <Dialog
                    handleClose={(value) => setOpenNonVoted(false)}
                    open={openNotVoted}
                    title={t("votingScreen.nonVotedDialog.title")}
                    ok={t("votingScreen.nonVotedDialog.ok")}
                    variant="softwarning"
                >
                    {stringToHtml(t("votingScreen.nonVotedDialog.content"))}
                </Dialog>
            ) : (
                <Dialog
                    handleClose={(value) => warnAllowContinue(value)}
                    open={openNotVoted}
                    title={t("votingScreen.nonVotedDialog.title")}
                    ok={t("votingScreen.nonVotedDialog.continue")}
                    cancel={t("votingScreen.nonVotedDialog.cancel")}
                    variant="action"
                >
                    {stringToHtml(t("votingScreen.nonVotedDialog.content"))}
                </Dialog>
            )}
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
