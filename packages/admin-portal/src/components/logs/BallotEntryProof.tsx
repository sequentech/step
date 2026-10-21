// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useQuery} from "@apollo/client"
import {Alert, Button, CircularProgress, Stack} from "@mui/material"
import {useTranslation} from "react-i18next"
import type {BallotLogTrust} from "@sequentech/ui-essentials"
import {BALLOT_BOX_ENTRY, BALLOT_BOX_KEYS} from "@/queries/BallotBoxLog"
import {IPermissions} from "@/types/keycloak"
import {
    BallotEntryEvidence,
    type IAdminBallotEvidence,
    type IBallotLogSelection,
} from "./BallotEntryEvidence"

export const BallotEntryProof: React.FC<{
    electionEventId: string
    electionId: string
    ballotId: string
    pseudonym: string
    selectedEntry: IBallotLogSelection
    zone: string
}> = ({electionEventId, electionId, ballotId, pseudonym, selectedEntry, zone}) => {
    const {t} = useTranslation()
    const context = {headers: {"x-hasura-role": IPermissions.LOGS_READ}}
    const entry = useQuery<{ballot_box_entry: {payload: IAdminBallotEvidence}}>(BALLOT_BOX_ENTRY, {
        variables: {electionEventId, electionId, ballotId},
        fetchPolicy: "no-cache",
        context,
    })
    const keys = useQuery<{ballot_box_keys: {payload: BallotLogTrust}}>(BALLOT_BOX_KEYS, {
        variables: {electionEventId, electionId, ballotId},
        fetchPolicy: "no-cache",
        context,
    })
    const proof = entry.data?.ballot_box_entry.payload
    const trust = keys.data?.ballot_box_keys.payload
    const loading = entry.loading || keys.loading
    const retry = () => {
        void entry.refetch().catch(() => undefined)
        void keys.refetch().catch(() => undefined)
    }
    const contextMatches =
        proof?.ballot_id === ballotId &&
        proof?.pseudonym === pseudonym &&
        trust?.election_id === electionId &&
        trust?.election_event_id === electionEventId
    return (
        <Stack spacing={2}>
            {loading ? (
                <CircularProgress aria-label={t("ballotPrivacy.checking")} />
            ) : entry.error || keys.error || !proof || !trust || !contextMatches ? (
                <>
                    <Alert severity="error">{t("ballotPrivacy.proofError")}</Alert>
                    <Button onClick={retry}>{t("ballotPrivacy.retry")}</Button>
                </>
            ) : (
                <BallotEntryEvidence
                    trust={trust}
                    proof={proof}
                    selectedEntry={selectedEntry}
                    zone={zone}
                    onRetry={retry}
                />
            )}
        </Stack>
    )
}
