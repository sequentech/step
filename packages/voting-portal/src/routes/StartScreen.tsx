// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {useTranslation} from "react-i18next"
import {Dialog, StartLayout} from "@sequentech/ui-essentials"
import {
    stringToHtml,
    translateFromPresentation,
    EStartScreenTitlePolicy,
    EElectionEventContestEncryptionPolicy,
    EDeclineToVotePolicy,
    areAllContestsAcclaimed,
} from "@sequentech/ui-core"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import StartActions from "../components/StartActions/StartActions"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {selectElectionById} from "../store/elections/electionsSlice"
import {CircularProgress} from "@mui/material"
import {useRootBackLink} from "../hooks/root-back-link"
import Stepper from "../components/Stepper"
import {selectBallotStyleByElectionId, showDemo} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {
    resetBallotSelection,
    selectBallotSelectionByElectionId,
    setAllBallotSelectionsDeclineToVote,
} from "../store/ballotSelections/ballotSelectionsSlice"
import {clearIsVoted, setDeclinedToVote, setIsVoted} from "../store/extra/extraSlice"
import {useEncryptBallotForReview} from "../hooks/useEncryptBallotForReview"
import {store} from "../store/store"

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
    const [openDeclineDialog, setOpenDeclineDialog] = useState(false)
    const dispatch = useAppDispatch()
    const navigate = useNavigate()
    const location = useLocation()
    const {encryptAndStoreBallot} = useEncryptBallotForReview()

    const titleObject = useMemo(() => {
        const startScreenTitlePolicy = election?.presentation?.start_screen_title_policy
        return startScreenTitlePolicy === EStartScreenTitlePolicy.ELECTION_EVENT
            ? electionEvent
            : election
    }, [election, electionEvent])

    const defaultLanguageCode =
        titleObject?.presentation?.language_conf?.default_language_code ??
        electionEvent?.presentation?.language_conf?.default_language_code
    const titleDescription = translateFromPresentation(titleObject, "description", i18n.language, {
        defaultLanguageCode,
    })

    useEffect(() => {
        if (!election || !titleObject) {
            navigate(backLink)
        }
    })

    useEffect(() => {
        if (!ballotStyle) {
            return
        }
        dispatch(
            resetBallotSelection({
                ballotStyle,
                force: true,
            })
        )
        dispatch(clearIsVoted())
    }, [ballotStyle])

    const declineToVotePolicy = election?.presentation?.decline_to_vote_policy
    const isMultiContest =
        ballotStyle?.ballot_eml.election_event_presentation?.contest_encryption_policy ===
        EElectionEventContestEncryptionPolicy.MULTIPLE_CONTESTS
    // Declining is casting a ballot, and a fully acclaimed election has no
    // ballot to cast.
    const isDeclineToVotePolicyEnabled =
        declineToVotePolicy === EDeclineToVotePolicy.ENABLED &&
        isMultiContest &&
        !areAllContestsAcclaimed(ballotStyle?.ballot_eml.contests)

    const confirmDeclineToVote = () => {
        if (!ballotStyle || !election) {
            return
        }

        setOpenDeclineDialog(false)
        dispatch(setAllBallotSelectionsDeclineToVote({ballotStyle}))
        dispatch(setDeclinedToVote(ballotStyle.election_id))
        dispatch(setIsVoted(ballotStyle.election_id))

        const declinedSelection = selectBallotSelectionByElectionId(ballotStyle.election_id)(
            store.getState()
        )
        if (!declinedSelection) {
            return
        }

        if (encryptAndStoreBallot(ballotStyle, declinedSelection, isMultiContest)) {
            navigate(
                `/tenant/${tenantId}/event/${eventId}/election/${election.id}/review${location.search}`
            )
        }
    }

    if (!election || !titleObject) {
        return <CircularProgress className="start-progress" aria-label={t("a11y.loading")} />
    }

    return (
        <StartLayout
            title={
                translateFromPresentation(titleObject, "name", i18n.language, {
                    defaultLanguageCode,
                }) ?? "-"
            }
            description={titleDescription ? stringToHtml(titleDescription) : undefined}
            steps={<Stepper selected={1} />}
            below={
                <StartActions
                    election={election}
                    isDeclineToVotePolicyEnabled={isDeclineToVotePolicyEnabled}
                    onDeclineToVoteClick={() => setOpenDeclineDialog(true)}
                />
            }
        >
            <Dialog
                variant="warning"
                open={showDemoDialog}
                ok={t("electionSelectionScreen.demoDialog.ok")}
                title={t("electionSelectionScreen.demoDialog.title")}
                handleClose={() => {
                    setShowDemoDialog(false)
                }}
                fullWidth
                className="demo-dialog"
            >
                {stringToHtml(t("electionSelectionScreen.demoDialog.content"))}
            </Dialog>

            {isDeclineToVotePolicyEnabled ? (
                <Dialog
                    className="decline-to-vote-dialog"
                    handleClose={(confirmed) => {
                        setOpenDeclineDialog(false)
                        if (confirmed) {
                            confirmDeclineToVote()
                        }
                    }}
                    open={openDeclineDialog}
                    title={t("startScreen.declineToVoteDialog.title")}
                    ok={t("startScreen.declineToVoteDialog.continue")}
                    cancel={t("startScreen.declineToVoteDialog.cancel")}
                    variant="info"
                >
                    {stringToHtml(t("startScreen.declineToVoteDialog.content"))}
                </Dialog>
            ) : null}
        </StartLayout>
    )
}

export default StartScreen
