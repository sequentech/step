// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {BallotSteps} from "@sequentech/ui-essentials"
import {selectBypassChooser} from "../store/extra/extraSlice"
import {useAppSelector} from "../store/hooks"

/**
 * The voter's breadcrumb, wired to this application's store.
 *
 * The steps themselves — their wording, their order, and what happens to the
 * numbering when an event bypasses the chooser — are `BallotSteps` in
 * `ui-essentials`, because the Election Architect's Ballot Preview has to draw the
 * same breadcrumb and was keeping a second list of labels to do it. All that is left
 * here is the one thing only this application can answer: whether the chooser is
 * bypassed.
 */
export default function Stepper({selected, warning}: {selected: number; warning?: boolean}) {
    const bypassElectionChooser = useAppSelector(selectBypassChooser())

    return (
        <BallotSteps
            selected={selected}
            withElectionList={!bypassElectionChooser}
            warning={warning}
        />
    )
}
