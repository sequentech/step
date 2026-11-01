// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {useTranslation} from "react-i18next"
import BreadCrumbSteps from "../components/BreadCrumbSteps/BreadCrumbSteps"

/**
 * The steps a voter walks, by the keys the portal has always used.
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
export enum EBallotStep {
    LIST = "breadcrumbSteps.electionList",
    INSTRUCTIONS = "breadcrumbSteps.instructions",
    BALLOT = "breadcrumbSteps.ballot",
    REVIEW = "breadcrumbSteps.review",
    CONFIRMATION = "breadcrumbSteps.confirmation",
}

/** What each number meant before the steps had names. */
const NUMBERED_STEPS = [
    EBallotStep.LIST,
    EBallotStep.BALLOT,
    EBallotStep.REVIEW,
    EBallotStep.CONFIRMATION,
]

export interface IBallotStepsProps {
    /**
     * Which step the voter is on, by name, or by number **as though the list were
     * always there**: `0` the ballot list, `1` the ballot, `2` review, `3`
     * confirmation. A number past those (the audit screen's `4`) marks no step.
     *
     * Either way a caller does not have to know whether this event skips the list
     * or gives the instructions a step of their own — the screen a voter is on is a
     * property of the flow, and which steps are drawn is a property of the event.
     */
    selected: EBallotStep | number
    /**
     * Whether this event offers the ballot list at all.
     *
     * An event with one election bypasses the chooser, and then there are three
     * steps rather than four; the portal reads this from its store as
     * `selectBypassChooser`, and a preview reads it from the plan.
     */
    withElectionList?: boolean
    /**
     * Whether the start screen's instructions are a step of their own, between the
     * list and the ballot. Without it they share the ballot's step, as they always
     * have; the portal reads this from the event's `instructions_step_policy`.
     */
    withInstructions?: boolean
    /** Draw the current step as a warning. The audit screen does. */
    warning?: boolean
    /** The list's accessible name. Defaults to `a11y.votingProgress`. */
    ariaLabel?: string
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
    withInstructions = false,
    warning,
    ariaLabel,
}: IBallotStepsProps): React.JSX.Element => {
    const {t} = useTranslation()
    const labels = [
        ...(withElectionList ? [EBallotStep.LIST] : []),
        ...(withInstructions ? [EBallotStep.INSTRUCTIONS] : []),
        EBallotStep.BALLOT,
        EBallotStep.REVIEW,
        EBallotStep.CONFIRMATION,
    ]
    const step = typeof selected === "number" ? NUMBERED_STEPS[selected] : selected
    // A step this event does not draw is shown as the one it is folded into: the
    // instructions into the ballot, and the list (support materials, for one,
    // passes it) into whatever comes first.
    const shown =
        step === EBallotStep.INSTRUCTIONS && !withInstructions
            ? EBallotStep.BALLOT
            : step === EBallotStep.LIST && !withElectionList
              ? labels[0]
              : step

    return (
        <BreadCrumbSteps
            labels={labels}
            selected={shown === undefined ? -1 : labels.indexOf(shown)}
            warning={warning}
            ariaLabel={ariaLabel ?? t("a11y.votingProgress")}
        />
    )
}
