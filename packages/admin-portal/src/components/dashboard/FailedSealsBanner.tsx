// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Alert, AlertTitle, Box} from "@mui/material"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useSealReadRole} from "@/hooks/useSealReadRole"
import {isSealAtClose} from "@/services/ballotBoxSealPolicy"
import {GET_FAILED_BALLOT_BOX_SEALS} from "@/queries/GetBallotBoxSeals"
import {sealText} from "@/services/ballotBoxSealErrors"
import type {GetFailedBallotBoxSealsQuery} from "@/types/ballotBoxSeal"

/**
 * The failed seals of an election event (VOTE-FREEZE, D6): an incident the
 * event's Dashboard and Publish tab show until it is resolved, so it isn't
 * seen only on the election's own Dashboard or in the Logs. Each line names
 * the election, the area and the reason. Nothing is shown when no seal
 * failed, when the event doesn't seal at close, or when the viewer can't
 * read seals.
 */
export const FailedSealsBanner: React.FC<{
    electionEventId?: string | null
    /** The event's presentation: only an event that seals at close is asked. */
    presentation?: unknown
}> = ({electionEventId, presentation}) => {
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const role = useSealReadRole()
    const {data} = useQuery<GetFailedBallotBoxSealsQuery>(GET_FAILED_BALLOT_BOX_SEALS, {
        variables: {electionEventId: electionEventId ?? ""},
        skip: !electionEventId || !role || !isSealAtClose(presentation),
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        context: role ? {headers: {"x-hasura-role": role}} : undefined,
    })
    const failed = data?.sequent_backend_ballot_box_seal ?? []
    if (!failed.length) return null
    return (
        <Alert severity="error" sx={{mb: 2}} className="failed-seals-banner">
            <AlertTitle>
                {t("dashboard.ballotBoxes.incident.title", {count: failed.length})}
            </AlertTitle>
            {t("dashboard.ballotBoxes.incident.body")}
            <Box component="ul" sx={{m: 0, mt: 1, pl: 2}}>
                {failed.map((seal) => (
                    <li key={seal.id}>
                        {t("dashboard.ballotBoxes.incident.line", {
                            election: seal.election_name ?? seal.election_id,
                            area: seal.area_name ?? seal.area?.name ?? seal.area_id,
                            reason: sealText(t, seal.failure_reason),
                        })}
                    </li>
                ))}
            </Box>
        </Alert>
    )
}
