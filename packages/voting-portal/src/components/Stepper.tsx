// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useParams} from "react-router-dom"
import {BallotSteps, EBallotStep} from "@sequentech/ui-essentials"
import {EInstructionsStepPolicy, IElectionEventPresentation} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {TenantEventType} from ".."
import {selectBypassChooser} from "../store/extra/extraSlice"
import {selectFirstBallotStyle} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {useAppSelector} from "../store/hooks"

/**
 * The voter's breadcrumb, wired to this application's store.
 *
 * The steps themselves — their wording, their order, and what happens to the
 * numbering when an event bypasses the chooser — are `BallotSteps` in
 * `ui-essentials`, because the Election Architect's Ballot Preview has to draw the
 * same breadcrumb and was keeping a second list of labels to do it. All that is left
 * here is what only this application can answer: whether the chooser is bypassed,
 * and whether the event gives the instructions a step of their own.
 */
export default function Stepper({
    selected,
    warning,
}: {
    selected: EBallotStep | number
    warning?: boolean
}) {
    const {t} = useTranslation()
    const {eventId} = useParams<TenantEventType>()
    const bypassElectionChooser = useAppSelector(selectBypassChooser())
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))

    const presentation: IElectionEventPresentation | undefined =
        ballotStyle?.ballot_eml.election_event_presentation ?? electionEvent?.presentation

    return (
        <BallotSteps
            selected={selected}
            withElectionList={!bypassElectionChooser}
            withInstructions={
                presentation?.instructions_step_policy === EInstructionsStepPolicy.OWN_STEP
            }
            warning={warning}
            ariaLabel={t("a11y.votingProgress")}
        />
    )
}
