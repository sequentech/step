// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useRef, useState} from "react"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {store} from "../store/store"
import {Box} from "@mui/material"
import {PageLimit, Icon, IconButton, theme, Dialog, VisuallyHidden} from "@sequentech/ui-essentials"
import {
    check_voting_error_dialog_bool,
    check_voting_not_allowed_next_bool,
    checkIsBlank,
    stringToHtml,
    isUndefined,
    translateFromPresentation,
    IContest,
    EElectionEventContestEncryptionPolicy,
    EBlankBallotsPolicy,
    BallotSelection,
    getDefaultVotingScreenBackPolicy,
    areAllContestsAcclaimed,
    getContestClassName,
    isAcclaimedContest,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {faCircleQuestion, faAngleLeft, faAngleRight} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {redirect, useLocation, useNavigate, useParams, useSubmit} from "react-router-dom"
import {
    selectBallotSelectionByElectionId,
    resetBallotSelection,
    setAllBallotSelectionsBlankBallot,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {clearDeclinedToVoteForElection, clearIsVoted, setIsVoted} from "../store/extra/extraSlice"
import {TenantEventType} from ".."
import {provideBallotService} from "../services/BallotService"
import {Question, contestTitleId} from "../components/Question/Question"
import {CircularProgress} from "@mui/material"
import {selectElectionById} from "../store/elections/electionsSlice"
import {useRootBackLink} from "../hooks/root-back-link"
import {VotingPortalError, VotingPortalErrorType} from "../services/VotingPortalError"
import Stepper from "../components/Stepper"
import {AuthContext} from "../providers/AuthContextProvider"
import {canVoteSomeElection} from "../store/castVotes/castVotesSlice"
import {IDecodedVoteContest} from "@sequentech/ui-core"
import {sortContestList} from "@sequentech/ui-core"
import {useEncryptBallotForReview} from "../hooks/useEncryptBallotForReview"
import {useBallotStyleSlates} from "../hooks/useBallotStyleSlates"
import {BallotSlatesProvider} from "../providers/BallotSlatesProvider"
import {SlateSelectionChooser} from "../components/SlateChooser/SlateSelectionChooser"
import {ESlateBallotTab, SlateBallotTabs} from "../components/SlateChooser/SlateBallotTabs"
import {SlateOfficeNavigator} from "../components/SlateChooser/SlateOfficeNavigator"
import {SlateBallotCount, SlateSelectionLabel} from "../components/SlateChooser/SlateBallotSummary"
import {useSlateClear} from "../components/SlateChooser/SlateClearFlow"
import {getEditContestId} from "../services/EditContest"
import {sortContestsInBallotOrder} from "../services/ContestOrder"

const StyledTitle = styled(Typography)<{component?: React.ElementType}>`
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
` as typeof Button

interface ActionButtonProps {
    handleNext: () => void
    handlePrev: () => void
    handleClearCustom?: () => void
    pageIndex?: number
    disableNext?: boolean
    disableClear?: boolean
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    handleNext,
    handlePrev,
    handleClearCustom,
    pageIndex,
    disableNext,
    disableClear,
}) => {
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const {tenantId, eventId, electionId} = useParams<TenantEventType & {electionId?: string}>()
    const location = useLocation()
    const navigate = useNavigate()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const dispatch = useAppDispatch()

    const votingScreenBackPolicy =
        election?.presentation?.voting_screen_back_policy ?? getDefaultVotingScreenBackPolicy()
    const exitLink =
        votingScreenBackPolicy === "start-screen"
            ? `/tenant/${tenantId}/event/${eventId}/election/${electionId}/start${location.search}`
            : `${backLink}${location.search}`

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
                className="clear-selection-button"
                sx={{
                    display: {sm: "none"},
                    width: "100%",
                }}
                variant="secondary"
                onClick={() => (handleClearCustom ? handleClearCustom() : handleClear())}
                disabled={disableClear}
            >
                <Box className="clear-selection-label">{t("votingScreen.clearButton")}</Box>
            </StyledButton>

            <ActionsContainer className="actions-container">
                <StyledButton
                    className="back-button"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    onClick={() => {
                        handlePrev()
                        if (!pageIndex || pageIndex <= 0) {
                            navigate(exitLink)
                        }
                    }}
                >
                    <Icon className="back-button-icon" icon={faAngleLeft} size="sm" />
                    <Box className="back-button-label">{t("votingScreen.backButton")}</Box>
                </StyledButton>

                <StyledButton
                    className="clear-selection-button"
                    sx={{
                        display: {xs: "none", sm: "block"},
                        width: {xs: "100%", sm: "200px"},
                    }}
                    variant="secondary"
                    onClick={() => (handleClearCustom ? handleClearCustom() : handleClear())}
                    disabled={disableClear}
                >
                    <Box className="clear-selection-label">{t("votingScreen.clearButton")}</Box>
                </StyledButton>

                <StyledButton
                    className="next-button"
                    sx={{width: {xs: "100%", sm: "200px"}}}
                    onClick={() => handleNext()}
                    disabled={disableNext}
                >
                    <Box className="next-button-label">{t("votingScreen.reviewButton")}</Box>
                    <Icon className="next-button-icon" icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
        </>
    )
}

interface IOfficeRequest {
    contestId?: string
}

interface ContestPaginationProps {
    ballotStyle: any
    contests: IContest[][]
    onSetDisableNext: (contest: any) => void
    onSetDecodedContests: (id: string) => (value: IDecodedVoteContest) => void
    encryptAndReview: () => void
    disableNextButton: (contests?: IContest[]) => boolean
    slateChooser?: React.ReactNode
    /** How many candidates the ballot holds, shown with the slates. */
    slateCount?: React.ReactNode
    /** What the ballot holds as a whole, above the individual candidates. */
    slateLabel?: React.ReactNode
    slateTab: ESlateBallotTab
    onSlateTabChange: (tab: ESlateBallotTab) => void
    /** The office the individual candidates should show. */
    officeRequest?: IOfficeRequest
    editContestId?: string
}

const ContestPagination: React.FC<ContestPaginationProps> = ({
    ballotStyle,
    contests,
    onSetDisableNext,
    onSetDecodedContests,
    encryptAndReview,
    disableNextButton,
    slateChooser,
    slateCount,
    slateLabel,
    slateTab,
    onSlateTabChange,
    officeRequest,
    editContestId,
}) => {
    const dispatch = useAppDispatch()
    const submit = useSubmit()

    const {t} = useTranslation()
    const contestsOrderType = ballotStyle?.ballot_eml.election_presentation?.contests_order
    const [pageIndex, setPageIndex] = useState(0)
    const pageAnnouncementRef = useRef<HTMLDivElement>(null)
    // A ballot with slates is cleared as a whole, after the voter sees what goes.
    const slateClear = useSlateClear(ballotStyle)
    const isFirstRender = useRef(true)

    useEffect(() => {
        if (isFirstRender.current) {
            isFirstRender.current = false
            return
        }
        pageAnnouncementRef.current?.focus()
    }, [pageIndex])

    // The review screen sends the voter back to edit one contest: its page is
    // opened and its heading takes the focus.
    const isEditContestShown = useRef(false)
    const [office, setOffice] = useState(0)
    const sortedContests = sortContestList(contests[pageIndex], contestsOrderType)
    useEffect(() => {
        const requested = (sortedContests ?? []).findIndex(
            (contest) => contest.id === officeRequest?.contestId
        )
        if (requested >= 0) {
            setOffice(requested)
        }
    }, [officeRequest])
    useEffect(() => {
        if (!editContestId || isEditContestShown.current) {
            return
        }
        const editPageIndex = contests.findIndex((page) =>
            page.some((contest) => contest.id === editContestId)
        )
        if (editPageIndex < 0) {
            return
        }
        if (editPageIndex !== pageIndex) {
            setPageIndex(editPageIndex)
            return
        }
        const editOffice = (sortedContests ?? []).findIndex(
            (contest) => contest.id === editContestId
        )
        if (slateChooser && editOffice >= 0 && editOffice !== office) {
            setOffice(editOffice)
            return
        }
        isEditContestShown.current = true
        const title = document.getElementById(contestTitleId(editContestId))
        if (title) {
            title.tabIndex = -1
            title.focus()
        }
    }, [contests, pageIndex, editContestId, office, sortedContests, slateChooser])
    const ballotSelectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle.election_id)
    )

    const {interpretContestSelection, interpretMultiContestSelection} = provideBallotService()

    const isMultiContest =
        ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ===
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    const errorSelectionState = useMemo(() => {
        if (!ballotSelectionState) {
            return []
        }
        let selectionState: BallotSelection = []
        try {
            if (isMultiContest) {
                selectionState = interpretMultiContestSelection(
                    ballotSelectionState,
                    ballotStyle.ballot_eml
                )
            } else {
                selectionState = interpretContestSelection(
                    ballotSelectionState,
                    ballotStyle.ballot_eml
                )
            }
        } catch (err) {
            console.log("Error was thrown by wasm interface: ", err)
            submit({error: VotingPortalErrorType.UNABLE_TO_ENCRYPT_BALLOT}, {method: "post"})
        }
        return selectionState
    }, [ballotSelectionState, isMultiContest, ballotStyle.ballot_eml])

    const handleNext = () => {
        // The contests of the later pages are only checked once they are
        // shown, so a ballot with several pages goes through them.
        if (slateChooser && slateTab === ESlateBallotTab.SLATES && contests.length > 1) {
            onSlateTabChange(ESlateBallotTab.CANDIDATES)
        } else if (pageIndex === contests.length - 1) {
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

    const renderContest = (contest: IContest, index: number) => (
        <Box
            key={contest.id}
            className={`contest-container contest-${index} ${getContestClassName(contest.external_id)}`.trim()}
        >
            <Question
                ballotStyle={ballotStyle}
                question={contest}
                isReview={false}
                setDisableNext={() => onSetDisableNext(contest.id)}
                setDecodedContests={onSetDecodedContests(contest.id)}
                errorSelectionState={errorSelectionState}
            />
        </Box>
    )

    const contestList = (
        <>
            {/* Paging through a multi-page ballot swaps the contests in place
                without a route change. Moving focus here both orients the voter
                and gets the new page number read out, so this is deliberately
                not also a live region — that would announce it twice. */}
            <VisuallyHidden
                className="contest-page-announcement"
                tabIndex={-1}
                ref={pageAnnouncementRef}
            >
                {t("a11y.stepOf", {current: pageIndex + 1, total: contests.length})}
            </VisuallyHidden>
            {slateChooser && sortedContests?.length ? (
                <SlateOfficeNavigator
                    contests={sortedContests}
                    selection={ballotSelectionState}
                    office={office}
                    onOfficeChange={setOffice}
                    renderContest={(contest) =>
                        renderContest(contest, sortedContests.indexOf(contest))
                    }
                />
            ) : (
                sortedContests?.map(renderContest)
            )}
        </>
    )

    return (
        <>
            {slateChooser ? (
                <>
                    {slateCount}
                    <SlateBallotTabs
                        value={slateTab}
                        onChange={onSlateTabChange}
                        slates={slateChooser}
                        candidates={
                            <>
                                {slateLabel}
                                {contestList}
                            </>
                        }
                    />
                </>
            ) : (
                contestList
            )}
            <ActionButtons
                handleNext={handleNext}
                handlePrev={handlePrev}
                handleClearCustom={slateChooser ? slateClear.requestClear : handleClear}
                disableClear={!!slateChooser && !slateClear.canClear}
                pageIndex={pageIndex}
                disableNext={disableNextButton(sortedContests) && contests.length !== 1}
            />
            {slateChooser ? slateClear.feedback : null}
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
    const [hasInvalidErrors, setHasInvalidErrors] = useState<boolean>(false)
    const [contestsPerPage, setContestsPerPage] = useState<IContest[][]>([])
    const location = useLocation()
    const editContestId = getEditContestId(location.state)
    const [officeRequest, setOfficeRequest] = useState<IOfficeRequest | undefined>()
    const [slateTab, setSlateTab] = useState(
        editContestId ? ESlateBallotTab.CANDIDATES : ESlateBallotTab.SLATES
    )

    const {encryptAndStoreBallot} = useEncryptBallotForReview()
    const election = useAppSelector(selectElectionById(String(electionId)))
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const defaultLanguageCode =
        election?.presentation?.language_conf?.default_language_code ??
        ballotStyle?.ballot_eml.election_event_presentation?.language_conf?.default_language_code
    const electionDescription = translateFromPresentation(election, "description", i18n.language, {
        defaultLanguageCode,
    })

    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle?.election_id ?? "")
    )

    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const dispatch = useAppDispatch()

    const submit = useSubmit()
    const slates = useBallotStyleSlates(ballotStyle)
    const hasSlates = !!slates.resolved && slates.resolved.slates.length > 0

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
    const disableNextButton = (contests = ballotStyle?.ballot_eml.contests): boolean => {
        return check_voting_not_allowed_next_bool(contests, decodedContests)
    }

    // if true, when the user click next, there will be a dialog that prompts
    // the user to confirm before going to the next screen
    const showNextDialog = () => {
        return check_voting_error_dialog_bool(ballotStyle?.ballot_eml.contests, decodedContests)
    }

    // whole-ballot blank ballots are only meaningful for MULTIPLE_CONTESTS
    // elections, and only when the admin has opted into the policy - this is
    // distinct from each contest's own (pre-existing) blank_vote_policy
    const isBlankBallotsPolicyEnabled = (): boolean => {
        const isMultiContest =
            ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ==
            EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
        return (
            election?.presentation?.blank_ballots_policy === EBlankBallotsPolicy.ENABLED &&
            isMultiContest
        )
    }

    const isWholeBallotBlank = (): boolean => {
        // Acclaimed contests are never encoded and never blank, so a ballot is
        // wholly blank when every contest the voter could actually fill in is.
        const contests = (ballotStyle?.ballot_eml.contests ?? []).filter(
            (contest) => !isAcclaimedContest(contest)
        )
        if (contests.length === 0 || !isBlankBallotsPolicyEnabled()) {
            return false
        }
        return contests.every((contest) => {
            const decoded = decodedContests[contest.id]
            return Boolean(decoded && checkIsBlank(decoded))
        })
    }

    const encryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        } else if (showNextDialog() || disableNextButton()) {
            setOpenNonVoted(true)

            const newHasInvalidErrors = Object.values(decodedContests).some(
                (a) => a?.invalid_errors.length > 0
            )
            setHasInvalidErrors(newHasInvalidErrors)
        } else {
            finallyEncryptAndReview()
        }
        dispatch(setIsVoted(electionId))
    }

    const finallyEncryptAndReview = () => {
        if (isUndefined(selectionState) || !ballotStyle) {
            return
        }

        // A fully acclaimed election produces no ballot, so there is nothing
        // to encrypt: the voter reviews what was decided by acclamation and
        // moves on.
        if (areAllContestsAcclaimed(ballotStyle.ballot_eml.contests)) {
            return submit(null, {method: "post"})
        }

        dispatch(clearDeclinedToVoteForElection(ballotStyle.election_id))

        // dispatch() updates the Redux store synchronously, but the
        // `selectionState` bound in this closure was captured by
        // useAppSelector on a prior render and won't reflect it - re-read
        // the store directly, mirroring StartScreen.tsx's confirmDeclineToVote().
        let selectionStateToEncrypt = selectionState
        if (isWholeBallotBlank()) {
            dispatch(setAllBallotSelectionsBlankBallot({ballotStyle}))
            selectionStateToEncrypt =
                selectBallotSelectionByElectionId(ballotStyle.election_id)(store.getState()) ??
                selectionState
        }

        const isMultiContest =
            ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ==
            EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS

        if (encryptAndStoreBallot(ballotStyle, selectionStateToEncrypt, isMultiContest)) {
            submit(null, {method: "post"})
        } else {
            submit({error: VotingPortalErrorType.UNABLE_TO_CAST_BALLOT}, {method: "post"})
        }
    }

    useEffect(() => {
        if (!election || !ballotStyle) {
            navigate(backLink)
        } else if (!selectionState || !canVote) {
            logout()
        } else if (electionId) {
            dispatch(clearDeclinedToVoteForElection(electionId))
        }
    }, [
        navigate,
        backLink,
        election,
        ballotStyle,
        selectionState,
        canVote,
        logout,
        electionId,
        dispatch,
    ])

    useEffect(() => {
        let minMaxGlobal = false
        let contestsPages = new Map<String, IContest[]>()
        let contests = sortContestsInBallotOrder(ballotStyle?.ballot_eml.contests ?? [])
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
        // A ballot with slates goes through its offices with the office
        // navigator instead of pages.
        const contestsAsArrays = hasSlates ? [contests] : Array.from(contestsPages.values())
        setContestsPerPage(contestsAsArrays)

        setDisableNext((state) => ({
            ...state,
            minmax_global: minMaxGlobal,
        }))
    }, [selectionState, ballotStyle, hasSlates])

    if (!ballotStyle || !election) {
        return <CircularProgress className="voting-progress" aria-label={t("a11y.loading")} />
    }

    const warnAllowContinue = (value: boolean) => {
        setOpenNonVoted(false)
        if (value) {
            finallyEncryptAndReview()
        }
    }

    return (
        <BallotSlatesProvider slates={slates.config}>
            <PageLimit maxWidth="lg" className="voting-screen screen">
                <Box marginTop="48px" className="stepper-box">
                    <Stepper selected={1} />
                </Box>
                <StyledTitle variant="h4" component="h1" className="title-container screen-title">
                    <Box className="selected-election-title">
                        {translateFromPresentation(election, "name", i18n.language, {
                            defaultLanguageCode,
                        }) ?? "-"}
                    </Box>
                    <IconButton
                        className="title-question"
                        buttonClassName="screen-help-button"
                        icon={faCircleQuestion}
                        sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                        fontSize="16px"
                        onClick={() => setOpenBallotHelp(true)}
                        ariaLabel={t("a11y.helpAbout", {
                            topic: t("votingScreen.ballotHelpDialog.title"),
                        })}
                    />
                    <Dialog
                        className="screen-help-dialog voting-help-dialog"
                        handleClose={() => setOpenBallotHelp(false)}
                        open={openBallotHelp}
                        title={t("votingScreen.ballotHelpDialog.title")}
                        ok={t("votingScreen.ballotHelpDialog.ok")}
                        variant="info"
                    >
                        {stringToHtml(t("votingScreen.ballotHelpDialog.content"))}
                    </Dialog>
                </StyledTitle>
                {electionDescription ? (
                    <Typography
                        className="description screen-description"
                        variant="body2"
                        component="div"
                        sx={{color: theme.palette.customGrey.main}}
                    >
                        {stringToHtml(electionDescription)}
                    </Typography>
                ) : null}

                <ContestPagination
                    slateChooser={
                        hasSlates && slates.resolved ? (
                            <SlateSelectionChooser
                                ballotStyle={ballotStyle}
                                slates={slates.resolved}
                                defaultLanguage={defaultLanguageCode}
                                onEditSelections={(slate) => {
                                    setSlateTab(ESlateBallotTab.CANDIDATES)
                                    setOfficeRequest({contestId: slate.contests[0]?.contest.id})
                                }}
                            />
                        ) : undefined
                    }
                    slateCount={<SlateBallotCount ballotStyle={ballotStyle} />}
                    slateLabel={
                        slates.resolved ? (
                            <SlateSelectionLabel
                                ballotStyle={ballotStyle}
                                slates={slates.resolved}
                                defaultLanguage={defaultLanguageCode}
                            />
                        ) : undefined
                    }
                    slateTab={slateTab}
                    officeRequest={officeRequest}
                    onSlateTabChange={setSlateTab}
                    editContestId={editContestId}
                    ballotStyle={ballotStyle}
                    contests={contestsPerPage}
                    onSetDisableNext={onSetDisableNext}
                    onSetDecodedContests={onSetDecodedContests}
                    encryptAndReview={encryptAndReview}
                    disableNextButton={disableNextButton}
                />

                {disableNextButton() ? (
                    <Dialog
                        className="ballot-validation-dialog"
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
                        className="ballot-validation-dialog"
                        handleClose={(value) => warnAllowContinue(value)}
                        open={openNotVoted}
                        title={t(
                            hasInvalidErrors
                                ? "votingScreen.nonVotedDialog.title"
                                : isWholeBallotBlank()
                                  ? "votingScreen.blankBallotDialog.title"
                                  : "votingScreen.warningDialog.title"
                        )}
                        ok={t(
                            hasInvalidErrors
                                ? "votingScreen.nonVotedDialog.continue"
                                : isWholeBallotBlank()
                                  ? "votingScreen.blankBallotDialog.continue"
                                  : "votingScreen.warningDialog.continue"
                        )}
                        cancel={t(
                            hasInvalidErrors
                                ? "votingScreen.nonVotedDialog.cancel"
                                : isWholeBallotBlank()
                                  ? "votingScreen.blankBallotDialog.cancel"
                                  : "votingScreen.warningDialog.cancel"
                        )}
                        variant="action"
                    >
                        {stringToHtml(
                            t(
                                hasInvalidErrors
                                    ? "votingScreen.nonVotedDialog.content"
                                    : isWholeBallotBlank()
                                      ? "votingScreen.blankBallotDialog.content"
                                      : "votingScreen.warningDialog.content"
                            )
                        )}
                    </Dialog>
                )}
            </PageLimit>
        </BallotSlatesProvider>
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
    const url = new URL(request.url)
    const search = url.search || ""

    return redirect(`../review${search}`)
}
