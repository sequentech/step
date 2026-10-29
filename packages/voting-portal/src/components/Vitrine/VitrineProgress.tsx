// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {selectBypassChooser} from "../../store/extra/extraSlice"
import {useAppSelector} from "../../store/hooks"
import {BOARD} from "./board"

const STEPS = ["breadcrumbSteps.ballot", "breadcrumbSteps.review", "breadcrumbSteps.confirmation"]

export interface IVitrineProgressProps {
    /** The step the voter is on, counting the election list as step 0, as the
     * portal's stepper does. */
    selected: number
    /** The current step's name. */
    label: string
}

/** Where the voter is in the voting steps, as the design board draws it: one
 * bar per step and "Step 3 of 3 · Confirmation". */
export const VitrineProgress: React.FC<IVitrineProgressProps> = ({selected, label}) => {
    const {t} = useTranslation()
    const bypassElectionChooser = useAppSelector(selectBypassChooser())
    const labels = bypassElectionChooser ? STEPS : ["breadcrumbSteps.electionList", ...STEPS]
    const index = Math.min(
        labels.length - 1,
        Math.max(0, bypassElectionChooser ? Math.max(0, selected - 1) : selected)
    )

    return (
        <Box
            className="vitrine-progress"
            role="group"
            aria-label={t("a11y.votingProgress")}
            sx={{display: "flex", flexDirection: "column", gap: "8px"}}
        >
            <Box
                className="vitrine-progress-bars"
                aria-hidden="true"
                sx={{
                    display: "grid",
                    gridTemplateColumns: `repeat(${labels.length}, minmax(0, 1fr))`,
                    gap: "6px",
                }}
            >
                {labels.map((label, i) => (
                    <Box
                        className="vitrine-progress-bar"
                        key={label}
                        sx={{height: "4px", bgcolor: i <= index ? BOARD.navy : BOARD.border}}
                    />
                ))}
            </Box>
            <Typography
                className="vitrine-progress-label"
                component="div"
                sx={{margin: 0, fontSize: "12px", color: BOARD.muted, letterSpacing: ".02em"}}
            >
                {t("vitrine.progress", {
                    current: index + 1,
                    total: labels.length,
                    step: label,
                })}
            </Typography>
        </Box>
    )
}
