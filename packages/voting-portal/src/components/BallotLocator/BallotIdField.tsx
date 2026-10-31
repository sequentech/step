// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Tracking ID field of the locator (design board TrackerSearch): it groups the
// ID in fours as the voter types, takes the short ID or the full one, and
// says so below it.

import React, {useState} from "react"
import {useTranslation} from "react-i18next"
import {Box, TextField} from "@mui/material"
import {stringToText} from "@sequentech/ui-core"
import {typedBallotId} from "../../services/vitrine/ballotId"
import {rejectedBallotId} from "../../services/vitrine/ballotTracker"

const HELP_ID = "ballot-id-help"
const ERROR_ID = "ballot-id-error"

export interface IBallotIdFieldProps {
    /** What the field shows: the ID in groups of four. */
    value: string
    onChange: (value: string) => void
    onSubmit: () => void
}

export const BallotIdField: React.FC<IBallotIdFieldProps> = ({value, onChange, onSubmit}) => {
    const {t} = useTranslation()
    const [rejected, setRejected] = useState(false)

    return (
        <>
            <TextField
                className="ballot-id-field"
                fullWidth
                value={value}
                label={t("a11y.ballotIdLabel")}
                placeholder={stringToText(t("ballotLocator.description"))}
                error={rejected}
                onChange={(event: React.ChangeEvent<HTMLInputElement>) => {
                    setRejected(rejectedBallotId(event.target.value))
                    onChange(typedBallotId(event.target.value))
                }}
                onKeyDown={(event) => {
                    if (event.key === "Enter") onSubmit()
                }}
                slotProps={{
                    htmlInput: {
                        "className": "ballot-id-input",
                        "autoComplete": "off",
                        "spellCheck": false,
                        "aria-describedby": rejected ? ERROR_ID : HELP_ID,
                    },
                    inputLabel: {shrink: true},
                }}
            />
            {/* The live region stays mounted and only its text changes: a region
                inserted at the same moment as its text is not reliably read. */}
            <Box
                className="ballot-id-error"
                id={ERROR_ID}
                role="alert"
                sx={{margin: "4px 14px 0", fontSize: 12, lineHeight: 1.66, color: "#991B1B"}}
            >
                {rejected ? t("ballotLocator.wrongFormatBallotId") : ""}
            </Box>
            {rejected ? null : (
                <Box
                    className="ballot-id-help"
                    id={HELP_ID}
                    sx={{margin: "4px 14px 0", fontSize: 12, lineHeight: 1.66, color: "#64748B"}}
                >
                    {t("ballotLocator.idHelp")}
                </Box>
            )}
        </>
    )
}
