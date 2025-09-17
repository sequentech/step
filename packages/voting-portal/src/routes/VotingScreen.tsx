// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {PageLimit, Icon, IconButton, theme, Dialog} from "@sequentech/ui-essentials"
import {
    check_voting_error_dialog_bool,
    check_voting_not_allowed_next_bool,
    stringToHtml,
    isUndefined,
    translateElection,
    IContest,
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    EElectionEventContestEncryptionPolicy,
} from "@sequentech/ui-core"
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
import {provideBallotService} from "../services/BallotService"
import {setAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {CircularProgress} from "@mui/material"
import {selectElectionById} from "../store/elections/electionsSlice"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import Stepper from "../components/Stepper"
import {AuthContext} from "../providers/AuthContextProvider"
import {canVoteSomeElection} from "../store/castVotes/castVotesSlice"
import {IDecodedVoteContest} from "@sequentech/ui-core"
import {sortContestList} from "@sequentech/ui-core"

const StyledLink = styled(RouterLink)`
    margin: auto 0;
    text-decoration: none;
    /* ensure the link contains only a single tabbable element: the button below */
    &:focus {
        outline: none;
    }
    & *[tabindex] {
        outline: none;
    }
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
    handlePrev: () => void
    handleClearCustom?: () => void
    pageIndex?: number
    disableNext?: boolean
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    handleNext,
    handlePrev,
    handleClearCustom,
    pageIndex,
    disableNext,
}) => {
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()

    function handleClear() {
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

    return (
        <>
            <StyledButton
                sx={{
                    display: {sm: "none"},
                    width: "100%",
                }}
                variant="secondary"
                onClick={() => (handleClearCustom ? handleClearCustom() : handleClear())}
            >
                <Box>{t("votingScreen.clearButton")}</Box>
            </StyledButton>

            <ActionsContainer>
                <StyledLink
                    to={pageIndex && pageIndex > 0 ? "" : backLink}
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
                    onClick={() => (handleClearCustom ? handleClearCustom() : handleClear())}
                >
                    <Box>{t("votingScreen.clearButton")}</Box>
                </StyledButton>

                <StyledButton
                    className="next-button"
                    sx={{width: {xs: "100%", sm: "200px"}}}
                    onClick={() => handleNext()}
                    disabled={disableNext}
                >
                    <Box>{t("votingScreen.reviewButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </>
    )
}

interface ContestPaginationProps {
    ballotStyle: any
    contests: IContest[][]
    onSetDisableNext: (contest: any) => void
    onSetDecodedContests: (id: string) => (value: IDecodedVoteContest) => void
    encryptAndReview: () => void
    disableNextButton: () => boolean
}

const ContestPagination: React.FC<ContestPaginationProps> = ({
    ballotStyle,
    contests,
    onSetDisableNext,
    onSetDecodedContests,
    encryptAndReview,
    disableNextButton,
}) => {
    const dispatch = useAppDispatch()

    const contestsOrderType = ballotStyle?.ballot_eml.election_presentation?.contests_order
    const [pageIndex, setPageIndex] = useState(0)
    const sortedContests = sortContestList(contests[pageIndex], contestsOrderType)
    const ballotSelectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )

    const {interpretContestSelection, interpretMultiContestSelection} = provideBallotService()

    const isMultiContest =
        ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ==
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    const errorSelectionState = useMemo(() => {
        if (!ballotSelectionState) {
            return []
        }
        return isMultiContest
            ? interpretMultiContestSelection(ballotSelectionState, ballotStyle.ballot_eml)
            : interpretContestSelection(ballotSelectionState, ballotStyle.ballot_eml)
    }, [ballotSelectionState, isMultiContest, ballotStyle.ballot_eml])

    const handleNext = () => {
        if (pageIndex === contests.length - 1) {
            encryptAndReview()
        } else {
            setPageIndex(pageIndex + 1)
        }
    }

    const handlePrev = () => {
        if (pageIndex > 0) {
            setPageIndex(pageIndex - 1)
        } else {
            dispatch(clearIsVoted())
        }
    }

    function handleClear() {
        if (ballotStyle) {
            contests[pageIndex].forEach((contest) => {
                dispatch(
                    resetBallotSelection({
                        ballotStyle,
                        force: true,
                        contestId: contest.id,
                    })
                )
            })
            if (pageIndex === 0) dispatch(clearIsVoted())
        }
    }

    return (
        <>
            {sortedContests &&
                sortedContests.map((contest, index) => (
                    <Box key={contest.id} className={`contest-${index}`}>
                        <Question
                            ballotStyle={ballotStyle}
                            question={contest}
                            isReview={false}
                            setDisableNext={() => onSetDisableNext(contest.id)}
                            setDecodedContests={onSetDecodedContests(contest.id)}
                            errorSelectionState={errorSelectionState}
                        />
                    </Box>
                ))}
            <ActionButtons
                handleNext={handleNext}
                handlePrev={handlePrev}
                handleClearCustom={handleClear}
                pageIndex={pageIndex}
                disableNext={disableNextButton() && contests.length !== 1}
            />
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
    const [contestsPerPage, setContestsPerPage] = useState<IContest[][]>([])

    const {
        encryptBallotSelection,
        encryptMultiBallotSelection,
        decodeAuditableBallot,
        decodeAuditableMultiBallot,
    } = provideBallotService()
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

    // if true, when the user click next, there will be a dialog that prompts
    // the user to confirm before going to the next screen
    const showNextDialog = () => {
        return check_voting_error_dialog_bool(ballotStyle?.ballot_eml.contests, decodedContests)
    }

    function handlePrev() {
        dispatch(clearIsVoted())
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
            const isMultiContest =
                ballotStyle.ballot_eml.election_event_presentation?.contest_encryption_policy ==
                EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

            const auditableBallot = isMultiContest
                ? encryptMultiBallotSelection(selectionState, ballotStyle.ballot_eml)
                : encryptBallotSelection(selectionState, ballotStyle.ballot_eml)

            dispatch(
                setAuditableBallot({
                    electionId: ballotStyle?.election_id ?? "",
                    auditableBallot,
                })
            )

            let decodedSelectionState = isMultiContest
                ? decodeAuditableMultiBallot(auditableBallot as IAuditableMultiBallot)
                : decodeAuditableBallot(auditableBallot as IAuditableSingleBallot)

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
        let contestsPages = new Map<String, IContest[]>()
        let contests = [...(ballotStyle?.ballot_eml.contests ?? [])].sort(
            (a, b) =>
                (a.presentation?.sort_order ?? Infinity) - (b.presentation?.sort_order ?? Infinity)
        )
        for (let contest of contests ?? []) {
            let countVotes = 0
            let selection = selectionState?.find((s) => s.contest_id === contest.id)
            for (let choice of selection?.choices ?? []) {
                if (choice.selected > -1) {
                    countVotes += choice.selected + 1
                }
            }
            let outOfRange = countVotes < contest.min_votes || countVotes > contest.max_votes
            minMaxGlobal = minMaxGlobal || outOfRange

            // Calculate contests pagination using the pagination_policy string identifier
            const contestPageName = contest.presentation?.pagination_policy || ""
            if (!contestsPages.has(contestPageName)) {
                contestsPages.set(contestPageName, [])
            }
            contestsPages.get(contestPageName)!.push(contest)
        }
        const contestsAsArrays = Array.from(contestsPages.values())
        setContestsPerPage(contestsAsArrays)

        setDisableNext((state) => ({
            ...state,
            minmax_global: minMaxGlobal,
        }))
    }, [selectionState, ballotStyle])

    if (!ballotStyle || !election) {
        return <CircularProgress />
    }

    const warnAllowContinue = (value: boolean) => {
        setOpenNonVoted(false)
        if (value) {
            finallyEncryptAndReview()
        }
    }

    return (
        <PageLimit maxWidth="lg" className="voting-screen screen">
            <Box marginTop="48px" className="stepper-box">
                <Stepper selected={1} />
            </Box>
            <StyledTitle variant="h4" className="title-container">
                <Box className="selected-election-title">
                    {translateElection(election, "name", i18n.language) ?? "-"}
                </Box>
                <IconButton
                    className="title-question"
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
                <Typography
                    className="description"
                    variant="body2"
                    sx={{color: theme.palette.customGrey.main}}
                >
                    {stringToHtml(translateElection(election, "description", i18n.language) ?? "-")}
                </Typography>
            ) : null}

            <ContestPagination
                ballotStyle={ballotStyle}
                contests={contestsPerPage}
                onSetDisableNext={onSetDisableNext}
                onSetDecodedContests={onSetDecodedContests}
                encryptAndReview={encryptAndReview}
                disableNextButton={disableNextButton}
            />

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
