// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, FormControlLabel, Switch} from "@mui/material"
import {useEventPresentation} from "@/hooks/useZonedFormat"
import {isSealAtClose} from "@/services/ballotBoxSealPolicy"
import {BallotBoxesCard, type BallotBoxesCardProps} from "./BallotBoxesCard"

/** Where the viewer's choice is remembered, in this browser only. */
export const BALLOT_BOXES_VISIBLE_KEY = "dashboard.ballotBoxes.visible"

/** Whether the Ballot boxes card is shown under the monitoring dashboards. */
export enum EBallotBoxesVisibility {
    SHOWN = "shown",
    HIDDEN = "hidden",
}

/** The remembered choice; shown unless hidden was stored and can be read. */
const readVisibility = (): EBallotBoxesVisibility => {
    try {
        return window.localStorage.getItem(BALLOT_BOXES_VISIBLE_KEY) ===
            EBallotBoxesVisibility.HIDDEN
            ? EBallotBoxesVisibility.HIDDEN
            : EBallotBoxesVisibility.SHOWN
    } catch {
        return EBallotBoxesVisibility.SHOWN
    }
}

/** Remembers the choice; a blocked storage keeps it for this view only. */
const writeVisibility = (visibility: EBallotBoxesVisibility) => {
    try {
        window.localStorage.setItem(BALLOT_BOXES_VISIBLE_KEY, visibility)
    } catch {
        // Not remembered: the next visit shows the ballot boxes again.
    }
}

/**
 * The Ballot boxes card under a Post's monitoring dashboards, with a switch
 * that hides it. Shown by default; the choice is remembered per viewer in
 * this browser. Nothing is shown when the event doesn't seal its ballot boxes.
 */
export const BallotBoxesSection: React.FC<BallotBoxesCardProps> = (props) => {
    const {t} = useTranslation()
    const presentation = useEventPresentation(props.electionEventId)
    const [visibility, setVisibility] = useState(readVisibility)
    if (!isSealAtClose(presentation)) return null
    const shown = visibility === EBallotBoxesVisibility.SHOWN
    const onChange = (checked: boolean) => {
        const next = checked ? EBallotBoxesVisibility.SHOWN : EBallotBoxesVisibility.HIDDEN
        setVisibility(next)
        writeVisibility(next)
    }
    return (
        <Box>
            <FormControlLabel
                control={
                    <Switch checked={shown} onChange={(event) => onChange(event.target.checked)} />
                }
                label={t("dashboard.ballotBoxes.show")}
            />
            {shown ? <BallotBoxesCard {...props} /> : null}
        </Box>
    )
}

export default BallotBoxesSection
