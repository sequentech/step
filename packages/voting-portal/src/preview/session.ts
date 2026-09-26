// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {isAcclaimedContest, type BallotSelection} from "@sequentech/ui-core"
import type {ScenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    updateBallotStyleAndSelection,
    type PreviewDocument,
} from "../routes/PreviewPublicationEvent"
import {provideBallotService} from "../services/BallotService"
import {
    checkIsExplicitBlankVote,
    checkIsInvalidVote,
    checkIsWriteIn,
} from "../services/ElectionConfigService"
import {
    selectBallotStyleByElectionId,
    type IBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {setIsVoted} from "../store/extra/extraSlice"
import {clearVoterSession, type AppDispatch, type RootState} from "../store/store"
import {isMultiContestStyle} from "./ballotPipeline"
import {PreviewScreen, type PreviewTarget} from "./screens"

export interface PreviewSession {
    snapshot: ScenarioSnapshot
    /** The screen whose state loading prepares; navigating afterwards keeps the session. */
    screen: PreviewScreen
}

export type EncryptForReview = (
    ballotStyle: IBallotStyle,
    selection: BallotSelection,
    isMultiContest: boolean
) => boolean

export function previewTarget(snapshot: ScenarioSnapshot): PreviewTarget {
    const style = snapshot.preview.ballot_styles.find(({area_id}) => area_id === snapshot.areaId)
    return {
        tenantId: snapshot.tenantId,
        eventId: snapshot.preview.election_event.id,
        electionId: style?.election_id,
    }
}

/** Replaces the voter session with the snapshot through the production preview loader. */
export function loadPreviewSnapshot(snapshot: ScenarioSnapshot, dispatch: AppDispatch) {
    dispatch(clearVoterSession())
    // Adapter boundary: the validated snapshot document is the publication preview document.
    updateBallotStyleAndSelection(
        snapshot.preview as unknown as PreviewDocument,
        snapshot.tenantId,
        snapshot.areaId,
        dispatch
    )
}

/**
 * A deterministic valid choice: the first `max(min_votes, 1)` selectable candidates of each
 * contest, ranked in order for preferential contests. Acclaimed contests stay unmarked.
 */
export function sampleSelection({ballot_eml}: IBallotStyle): BallotSelection {
    const {isPreferential} = provideBallotService()
    return ballot_eml.contests.map((contest) => {
        const selectable = contest.candidates.filter(
            (candidate) =>
                !checkIsInvalidVote(candidate) &&
                !checkIsExplicitBlankVote(candidate) &&
                !checkIsWriteIn(candidate)
        )
        const count = isAcclaimedContest(contest)
            ? 0
            : Math.min(Math.max(contest.min_votes, 1), contest.max_votes, selectable.length)
        const ranked = isPreferential(contest.counting_algorithm)
        const marks = new Map(
            selectable.slice(0, count).map((candidate, index) => [candidate.id, ranked ? index : 0])
        )
        return {
            contest_id: contest.id,
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: contest.candidates.map(({id}) => ({id, selected: marks.get(id) ?? -1})),
        }
    })
}

/**
 * Review and confirmation expect a ballot the voter already encrypted, so opening them
 * encrypts the sample selection with the production hook first.
 */
export function preparePreviewScreen(
    session: PreviewSession,
    state: RootState,
    dispatch: AppDispatch,
    encrypt: EncryptForReview
) {
    if (session.screen !== PreviewScreen.REVIEW && session.screen !== PreviewScreen.CONFIRMATION)
        return
    const {electionId} = previewTarget(session.snapshot)
    const style = electionId ? selectBallotStyleByElectionId(electionId)(state) : undefined
    if (!style)
        throw new Error(`The snapshot has no ballot to prepare the ${session.screen} screen`)
    if (!encrypt(style, sampleSelection(style), isMultiContestStyle(style)))
        throw new Error(`The sample ballot for the ${session.screen} screen could not be encrypted`)
    dispatch(setIsVoted(style.election_id))
}
