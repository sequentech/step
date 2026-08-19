// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"

import BreadCrumbSteps from "../components/BreadCrumbSteps/BreadCrumbSteps"

/**
 * The four steps a voter walks, in the portal's own words.
 *
 * The keys are the portal's (`src/translations/en.ts`), so a deployment that has
 * translated them keeps its wording. The second argument to each `t` is the English
 * the portal ships, which is what a host without those keys gets: the Election
 * Architect's Ballot Preview draws this stepper and has its own catalogue, and a
 * preview showing `breadcrumbSteps.ballot` where a voter sees "Ballot" would be
 * worse than a preview showing English.
 */
interface StepLabel {
    /** The portal's catalogue key. */
    key: string
    /** What a host without that key shows instead. */
    english: string
}

const LIST: StepLabel = {key: "breadcrumbSteps.electionList", english: "Ballots"}
const BALLOT: StepLabel = {key: "breadcrumbSteps.ballot", english: "Ballot"}
const REVIEW: StepLabel = {key: "breadcrumbSteps.review", english: "Review"}
const CONFIRMATION: StepLabel = {key: "breadcrumbSteps.confirmation", english: "Confirm"}

export interface IBallotStepsProps {
    /**
     * Which step the voter is on, counted **as though the list were always there**:
     * `0` the ballot list, `1` the ballot, `2` review, `3` confirmation.
     *
     * Counted that way so a caller does not have to know whether this event skips
     * the list — the numbering of a screen is a property of the flow, and the list
     * is a property of the event. `withElectionList={false}` shifts them down here.
     */
    selected: number
    /**
     * Whether this event offers the ballot list at all.
     *
     * An event with one election bypasses the chooser, and then there are three
     * steps rather than four; the portal reads this from its store as
     * `selectBypassChooser`, and a preview reads it from the plan.
     */
    withElectionList?: boolean
    /** Draw the current step as a warning. The audit screen does. */
    warning?: boolean
}

/**
 * The voter's breadcrumb.
 *
 * Lifted out of the portal's `components/Stepper.tsx`, which is now this component
 * plus the one `useAppSelector` that answers `withElectionList`. It is here because
 * the Ballot Preview has to draw the same four steps with the same words, and the
 * alternative — a second list of labels in the wizard — drifts the moment somebody
 * renames one.
 */
export const BallotSteps = ({
    selected,
    withElectionList = true,
    warning,
}: IBallotStepsProps): React.JSX.Element => {
    const {t} = useTranslation()

    const order: Array<StepLabel> = withElectionList
        ? [LIST, BALLOT, REVIEW, CONFIRMATION]
        : [BALLOT, REVIEW, CONFIRMATION]

    // `BreadCrumbSteps` translates what it is handed, which is why the portal passes
    // it bare keys. Resolving them here instead is what lets the fallbacks above
    // exist; `t` on already-English text returns that text, so this stays a no-op
    // for it.
    const labels = order.map(({key, english}) => t(key, {defaultValue: english}))

    return (
        <BreadCrumbSteps
            labels={labels}
            selected={withElectionList ? selected : Math.max(0, selected - 1)}
            warning={warning}
        />
    )
}
