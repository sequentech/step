// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import BreadCrumbSteps from "../components/BreadCrumbSteps/BreadCrumbSteps"

/**
 * The four steps a voter walks, by the keys the portal has always used.
 *
 * Keys and not text: `BreadCrumbSteps` translates each label it is handed, which is how
 * the portal's `Stepper` passed these before this component existed, and it is why the
 * strings stay in `voting-portal/src/translations/<lng>.ts` where clients override them.
 *
 * There was an English copy of these four words here, added on the theory that a host
 * without `breadcrumbSteps.*` would draw raw keys. Every host has them — the wizard
 * vendors the portal's catalogue and hands it to the preview — so the copy bought nothing
 * and cost the Spanish. `EA-F2-053`.
 */
const LIST = "breadcrumbSteps.electionList"
const BALLOT = "breadcrumbSteps.ballot"
const REVIEW = "breadcrumbSteps.review"
const CONFIRMATION = "breadcrumbSteps.confirmation"

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
    const labels = withElectionList
        ? [LIST, BALLOT, REVIEW, CONFIRMATION]
        : [BALLOT, REVIEW, CONFIRMATION]

    return (
        <BreadCrumbSteps
            labels={labels}
            selected={withElectionList ? selected : Math.max(0, selected - 1)}
            warning={warning}
        />
    )
}
