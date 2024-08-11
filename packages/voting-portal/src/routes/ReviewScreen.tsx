// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Link as RouterLink, useNavigate, useParams, redirect, useLocation} from "react-router-dom"
import {IBallotStyle, selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {Box} from "@mui/material"
import {
    PageLimit,
    Icon,
    IconButton,
    theme,
    BallotHash,
    Dialog,
    WarnBox,
} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    EVotingStatus,
    IElectionEventStatus,
    IAuditableBallot,
} from "@sequentech/ui-core"
import {styled} from "@mui/material/styles"
import Typography from "@mui/material/Typography"
import {
    faCircleQuestion,
    faAngleLeft,
    faAngleRight,
    faFire,
} from "@fortawesome/free-solid-svg-icons"
import {useTranslation} from "react-i18next"
import Button from "@mui/material/Button"
import {selectAuditableBallot} from "../store/auditableBallots/auditableBallotsSlice"
import {Question} from "../components/Question/Question"
import {useMutation, useQuery} from "@apollo/client"
import {INSERT_CAST_VOTE} from "../queries/InsertCastVote"
import {GetElectionEventQuery, InsertCastVoteMutation} from "../gql/graphql"
import {CircularProgress} from "@mui/material"
import {provideBallotService} from "../services/BallotService"
import {ICastVote, addCastVotes} from "../store/castVotes/castVotesSlice"
import {TenantEventType} from ".."
import {useRootBackLink} from "../hooks/root-back-link"
import {
    CastBallotsErrorType,
    VotingPortalError,
    VotingPortalErrorType,
    WasmCastBallotsErrorType,
} from "../services/VotingPortalError"
import {IBallotError} from "../types/errors"
import {GET_ELECTION_EVENT} from "../queries/GetElectionEvent"
import Stepper from "../components/Stepper"
import {selectBallotSelectionByElectionId} from "../store/ballotSelections/ballotSelectionsSlice"
import {AuthContext} from "../providers/AuthContextProvider"
import {sortContestList, hashBallot} from "@sequentech/ui-core"

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
    ballotStyle: IBallotStyle
    auditableBallot: IAuditableBallot
    hideAudit: boolean
    castVoteConfirmModal: boolean
    ballotId: string
    setErrorMsg: (msg: CastBallotsErrorType) => void
}

const ActionButtons: React.FC<ActionButtonProps> = ({
    ballotStyle,
    auditableBallot,
    hideAudit,
    castVoteConfirmModal,
    ballotId,
    setErrorMsg,
}) => {
    const dispatch = useAppDispatch()
    const [insertCastVote] = useMutation<InsertCastVoteMutation>(INSERT_CAST_VOTE)
    const {t} = useTranslation()
    const navigate = useNavigate()
    const location = useLocation()
    const [auditBallotHelp, setAuditBallotHelp] = useState<boolean>(false)
    const [isCastingBallot, setIsCastingBallot] = React.useState<boolean>(false)
    const [isConfirmCastVoteModal, setConfirmCastVoteModal] = React.useState<boolean>(false)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const {toHashableBallot} = provideBallotService()
    const isDemo = !!ballotStyle?.ballot_eml?.public_key?.is_demo

    const {refetch: refetchElectionEvent, error: errorFetchingElectionEvent} =
        useQuery<GetElectionEventQuery>(GET_ELECTION_EVENT, {
            variables: {
                electionEventId: eventId,
                tenantId,
            },
        })

    const handleClose = (value: boolean) => {
        setAuditBallotHelp(false)
        if (value) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/audit${location.search}`
            )
        }
    }

    const handleCloseCastVoteDialog = (value: boolean) => {
        setConfirmCastVoteModal(false)
        if (value) {
            castBallotAction()
        }
    }

    const fakeCastVote = (): ICastVote => ({
        id: eventId ?? "",
        tenant_id: tenantId ?? "",
        election_id: eventId,
        area_id: eventId,
        created_at: null,
        last_updated_at: null,
        annotations: null,
        labels: null,
        content: "",
        cast_ballot_signature: "",
        voter_id_string: null,
        election_event_id: eventId ?? "",
    })

    const castBallotAction = async () => {
        if (isDemo) {
            console.log("faking casting demo vote")
            const newCastVote = fakeCastVote()
            dispatch(addCastVotes([newCastVote]))
            return
        }
        setIsCastingBallot(true)

        try {
            const {data} = await refetchElectionEvent()

            if (!(data && data.sequent_backend_election_event.length > 0)) {
                setIsCastingBallot(false)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.LOAD_ELECTION_EVENT}`))
                console.error(t(`reviewScreen.error.${CastBallotsErrorType.LOAD_ELECTION_EVENT}`))
                return
            }

            const record = data?.sequent_backend_election_event?.[0]
            const eventStatus = record?.status as IElectionEventStatus | undefined

            if (eventStatus?.voting_status !== EVotingStatus.OPEN) {
                setIsCastingBallot(false)
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.ELECTION_EVENT_NOT_OPEN}`))
                return
            }

            const hashableBallot = toHashableBallot({...auditableBallot, contests: ["sdfdfssdf"]})

            let result = await insertCastVote({
                variables: {
                    electionId: ballotStyle.election_id,
                    ballotId,
                    content: JSON.stringify(hashableBallot),
                },
            })
            let newCastVote = result.data?.insert_cast_vote
            if (newCastVote) {
                dispatch(addCastVotes([newCastVote]))
            }

            return redirect(`../confirmation`)
        } catch (error) {
            setIsCastingBallot(false)
            // dispatch(clearBallot())
            const ballotError = error as IBallotError
            if (errorFetchingElectionEvent) {
                setErrorMsg(t(VotingPortalErrorType.UNABLE_TO_FETCH_DATA))
            }
            if (ballotError.error_type) {
                setErrorMsg(t(`reviewScreen.error.${WasmCastBallotsErrorType[ballotError.error_type]}`))
            }
            console.log(ballotError?.error_msg || error)
            console.log(`error casting vote: ${ballotStyle.election_id}`)
            return
        }
    }

    return (
        <Box sx={{marginBottom: "10px", marginTop: "10px"}}>
            <StyledButton
                sx={{display: {xs: "none", sm: "none"}, marginBottom: "2px", width: "100%"}}
                variant="warning"
                onClick={() => setAuditBallotHelp(true)}
            >
                <Icon icon={faFire} size="sm" />
                <Box>{t("reviewScreen.auditButton")}</Box>
            </StyledButton>
            <Dialog
                handleClose={handleClose}
                open={auditBallotHelp}
                title={t("reviewScreen.auditBallotHelpDialog.title")}
                ok={t("reviewScreen.auditBallotHelpDialog.ok")}
                cancel={t("reviewScreen.auditBallotHelpDialog.cancel")}
                variant="warning"
            >
                {stringToHtml(t("reviewScreen.auditBallotHelpDialog.content"))}
            </Dialog>
            <ActionsContainer>
                <StyledLink
                    to={`/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/vote${location.search}`}
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                >
                    <StyledButton sx={{width: {xs: "100%", sm: "200px"}}}>
                        <Icon icon={faAngleLeft} size="sm" />
                        <Box>{t("reviewScreen.backButton")}</Box>
                    </StyledButton>
                </StyledLink>
                {hideAudit ? null : (
                    <StyledButton
                        sx={{width: {xs: "100%", sm: "200px"}, display: {xs: "none", sm: "flex"}}}
                        variant="warning"
                        onClick={() => setAuditBallotHelp(true)}
                    >
                        <Icon icon={faFire} size="sm" />
                        <Box>{t("reviewScreen.auditButton")}</Box>
                    </StyledButton>
                )}
                <StyledButton
                    className="cast-ballot-button"
                    sx={{margin: "auto 0", width: {xs: "100%", sm: "200px"}}}
                    disabled={isCastingBallot}
                    onClick={() =>
                        castVoteConfirmModal ? setConfirmCastVoteModal(true) : castBallotAction()
                    }
                >
                    <Box>{t("reviewScreen.castBallotButton")}</Box>
                    <Icon icon={faAngleRight} size="sm" />
                </StyledButton>
            </ActionsContainer>
            <Dialog
                handleClose={handleCloseCastVoteDialog}
                open={isConfirmCastVoteModal}
                title={t("reviewScreen.confirmCastVoteDialog.title")}
                ok={t("reviewScreen.confirmCastVoteDialog.ok")}
                cancel={t("reviewScreen.confirmCastVoteDialog.cancel")}
                variant="info"
            >
                {stringToHtml(t("reviewScreen.confirmCastVoteDialog.content"))}
            </Dialog>
        </Box>
    )
}

export const ReviewScreen: React.FC = () => {
    const {electionId} = useParams<{electionId?: string}>()
    const ballotStyle = useAppSelector(selectBallotStyleByElectionId(String(electionId)))
    const auditableBallot = useAppSelector(selectAuditableBallot(String(electionId)))
    const [openBallotIdHelp, setOpenBallotIdHelp] = useState(false)
    const [openReviewScreenHelp, setReviewScreenHelp] = useState(false)
    const {t} = useTranslation()
    const backLink = useRootBackLink()
    const navigate = useNavigate()
    const {logout} = useContext(AuthContext)
    const {tenantId, eventId} = useParams<TenantEventType>()
    const [errorMsg, setErrorMsg] = useState<CastBallotsErrorType>()

    const hideAudit = ballotStyle?.ballot_eml?.election_event_presentation?.hide_audit ?? false
    const castVoteConfirmModal =
        ballotStyle?.ballot_eml?.election_presentation?.cast_vote_confirm ?? false
    const ballotId = auditableBallot && hashBallot(auditableBallot)

    if (ballotId && auditableBallot?.ballot_hash && ballotId !== auditableBallot.ballot_hash) {
        console.error(
            t("errors.encoding.writeInCharsExceeded", {
                ballotId,
                auditableBallotHash: auditableBallot.ballot_hash,
            })
        )
        setErrorMsg(
            t("errors.encoding.writeInCharsExceeded", {
                ballotId,
                auditableBallotHash: auditableBallot.ballot_hash,
            })
        )
    }

    const selectionState = useAppSelector(
        selectBallotSelectionByElectionId(ballotStyle?.election_id ?? "")
    )

    function handleCloseDialog(val: boolean) {
        setOpenBallotIdHelp(false)

        if (val) {
            if (ballotStyle && tenantId && eventId) {
                navigate(
                    `/tenant/${tenantId}/event/${eventId}/election/${ballotStyle.election_id}/audit`
                )
            } else {
                setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NO_BALLOT_STYLE}`))
            }
        }
    }

    useEffect(() => {
        if (!ballotStyle) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NO_BALLOT_STYLE}`))
            //TODO: on confirm dialog - navigate(backLink)
        } else if (!auditableBallot) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NO_AUDITABLE_BALLOT}`))
            //TODO: on confirm dialog - navigate(backLink)
        } else if (!selectionState) {
            setErrorMsg(t(`reviewScreen.error.${CastBallotsErrorType.NO_BALLOT_SELECTION}`))
            //TODO: on confirm dialog - logout()
        }
    })

    if (!ballotStyle || !auditableBallot) {
        return <CircularProgress />
    }

    const contestsOrderType = ballotStyle?.ballot_eml.election_presentation?.contests_order
    const contests = sortContestList(ballotStyle.ballot_eml.contests, contestsOrderType)

    return (
        <PageLimit maxWidth="lg" className="review-screen screen">
            {hideAudit ? null : (
                <BallotHash hash={ballotId || ""} onHelpClick={() => setOpenBallotIdHelp(true)} />
            )}
            {errorMsg && <WarnBox variant="warning">{errorMsg}</WarnBox>}
            <Dialog
                handleClose={handleCloseDialog}
                open={openBallotIdHelp}
                title={t("reviewScreen.ballotIdHelpDialog.title")}
                ok={t("reviewScreen.ballotIdHelpDialog.ok")}
                cancel={t("reviewScreen.ballotIdHelpDialog.cancel")}
                variant="info"
            >
                {stringToHtml(t("reviewScreen.ballotIdHelpDialog.content"))}
            </Dialog>
            <Box marginTop="48px">
                <Stepper selected={2} />
            </Box>
            <StyledTitle variant="h4" fontSize="24px" fontWeight="bold" sx={{margin: 0}}>
                <Box>{t("reviewScreen.title")}</Box>
                <IconButton
                    icon={faCircleQuestion}
                    sx={{fontSize: "unset", lineHeight: "unset", paddingBottom: "2px"}}
                    fontSize="16px"
                    onClick={() => setReviewScreenHelp(true)}
                />
                <Dialog
                    handleClose={() => setReviewScreenHelp(false)}
                    open={openReviewScreenHelp}
                    title={t("reviewScreen.reviewScreenHelpDialog.title")}
                    ok={t("reviewScreen.reviewScreenHelpDialog.ok")}
                    variant="info"
                >
                    {stringToHtml(t("reviewScreen.reviewScreenHelpDialog.content"))}
                </Dialog>
            </StyledTitle>
            <Typography variant="body2" sx={{color: theme.palette.customGrey.main}}>
                {stringToHtml(
                    hideAudit ? t("reviewScreen.descriptionNoAudit") : t("reviewScreen.description")
                )}
            </Typography>
            {contests.map((question, index) => (
                <Question
                    ballotStyle={ballotStyle}
                    question={question}
                    key={index}
                    isReview={true}
                    setDecodedContests={() => undefined}
                />
            ))}
            <ActionButtons
                ballotStyle={ballotStyle}
                auditableBallot={auditableBallot}
                hideAudit={hideAudit}
                castVoteConfirmModal={castVoteConfirmModal}
                ballotId={ballotId ?? ""}
                setErrorMsg={setErrorMsg}
            />
        </PageLimit>
    )
}

export default ReviewScreen
