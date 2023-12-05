// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useGetOne} from "react-admin"

import {Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {InputAdornment, TextField} from "@mui/material"
import CalendarMonthIcon from "@mui/icons-material/CalendarMonth"
import {useTranslation} from "react-i18next"

export const TallyStartDate: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()

    const {t} = useTranslation()

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    return (
        <TextField
            sx={{width: "100%"}}
            disabled
            label={t("tally.common.date")}
            defaultValue={data?.created_at}
            InputProps={{
                startAdornment: (
                    <InputAdornment position="end">
                        <CalendarMonthIcon />
                    </InputAdornment>
                ),
            }}
            variant="standard"
        />
    )
}
