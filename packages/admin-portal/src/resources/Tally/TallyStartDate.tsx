// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useGetOne} from "react-admin"

import {Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {InputAdornment, TextField} from "@mui/material"
import CalendarMonthIcon from "@mui/icons-material/CalendarMonth"
import {useTranslation} from "react-i18next"
import {useEventZonedFormat} from "@/hooks/useZonedFormat"

export const TallyStartDate: React.FC = () => {
    const {tallyId} = useElectionEventTallyStore()

    const {t} = useTranslation()

    const {data} = useGetOne<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            id: tallyId,
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const format = useEventZonedFormat(data?.election_event_id)

    return (
        <TextField
            sx={{width: "100%"}}
            disabled
            label={String(t("tally.common.date"))}
            value={format.format(data?.created_at)}
            InputProps={{
                endAdornment: (
                    <InputAdornment position="end">
                        <CalendarMonthIcon />
                    </InputAdornment>
                ),
            }}
            variant="outlined"
        />
    )
}
