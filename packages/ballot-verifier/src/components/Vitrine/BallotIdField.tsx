// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The field where the user gives a Tracking ID: it groups the digits as they are
// typed or pasted, and takes only a full ID, since a short one could be matched
// by a birthday search.

import React from "react"
import TextField from "@mui/material/TextField"
import {useTranslation} from "react-i18next"
import {ballotIdInput, EBallotIdInput, typedBallotId} from "../../services/vitrine/ballotId"

export interface IBallotIdFieldProps {
    /** The ID as the field shows it: grouped. */
    value: string
    onChange: (value: string) => void
}

export const BallotIdField: React.FC<IBallotIdFieldProps> = ({value, onChange}) => {
    const {t} = useTranslation()
    const input = ballotIdInput(value)
    const help = (): string => {
        switch (input) {
            case EBallotIdInput.SHORT:
                return t("ballotIdField.shortError")
            case EBallotIdInput.INVALID:
                return t("ballotIdField.formatError")
            default:
                return t("ballotIdField.help")
        }
    }

    return (
        <TextField
            label={t("ballotIdField.label")}
            placeholder={t("ballotIdField.placeholder")}
            InputLabelProps={{shrink: true}}
            value={value}
            onChange={(event) => onChange(typedBallotId(event.target.value))}
            error={input === EBallotIdInput.SHORT || input === EBallotIdInput.INVALID}
            helperText={help()}
            inputProps={{"data-testid": "ballot-id", "spellCheck": false, "autoComplete": "off"}}
            sx={{maxWidth: "630px", width: "100%"}}
        />
    )
}
