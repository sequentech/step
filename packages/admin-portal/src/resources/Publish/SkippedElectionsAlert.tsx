// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Alert, AlertTitle, Box} from "@mui/material"
import {useTranslation} from "react-i18next"

/** Why the server left an election as it was in an event-wide change. */
export enum ESkippedElectionReason {
    /** With Seal at close, closed voting stays closed (VOTE-FREEZE). */
    BALLOT_BOX_SEAL_POLICY = "ballot-box-seal-policy",
}

export interface ISkippedElection {
    election_id: string
    election_name?: string | null
    reason: string
}

/** The explanation of one skipped election, in the viewer's language. */
export const skippedElectionText = (
    t: (key: string, options?: Record<string, unknown>) => string,
    {election_id, election_name, reason}: ISkippedElection
): string => {
    const name = election_name || election_id
    return reason === ESkippedElectionReason.BALLOT_BOX_SEAL_POLICY
        ? t("publish.skippedElections.ballotBoxSealPolicy", {name})
        : t("publish.skippedElections.other", {name, reason})
}

/**
 * The elections an event-wide Start or Resume left as they were, each with
 * the server's reason, until the administrator dismisses the list.
 */
export const SkippedElectionsAlert: React.FC<{
    skipped: ISkippedElection[]
    onClose: () => void
}> = ({skipped, onClose}) => {
    const {t} = useTranslation()
    if (!skipped.length) return null
    return (
        <Alert
            severity="info"
            onClose={onClose}
            closeText={t("publish.skippedElections.dismiss")}
            sx={{mb: 2}}
        >
            <AlertTitle>{t("publish.skippedElections.title", {count: skipped.length})}</AlertTitle>
            <Box component="ul" sx={{m: 0, pl: 2}}>
                {skipped.map((election) => (
                    <li key={election.election_id}>{skippedElectionText(t, election)}</li>
                ))}
            </Box>
        </Alert>
    )
}
