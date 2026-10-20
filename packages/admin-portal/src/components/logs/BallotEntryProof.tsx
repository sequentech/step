// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useQuery} from "@apollo/client"
import {Alert, Button, CircularProgress, Stack} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    BallotLogProof,
    type BallotLogEvidence,
    type BallotLogTrust,
} from "@sequentech/ui-essentials"
import {BALLOT_BOX_ENTRY, BALLOT_BOX_KEYS} from "@/queries/BallotBoxLog"
import {IPermissions} from "@/types/keycloak"

export const BallotEntryProof: React.FC<{
    electionEventId: string
    electionId: string
    ballotId: string
}> = ({electionEventId, electionId, ballotId}) => {
    const {t} = useTranslation()
    const context = {headers: {"x-hasura-role": IPermissions.LOGS_READ}}
    const entry = useQuery<{ballot_box_entry: {payload: BallotLogEvidence}}>(BALLOT_BOX_ENTRY, {
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
    const contextMatches =
        proof?.ballot_id === ballotId &&
        trust?.election_id === electionId &&
        trust?.election_event_id === electionEventId
    return (
        <Stack spacing={2}>
            {loading ? (
                <CircularProgress aria-label={t("ballotPrivacy.checking")} />
            ) : entry.error || keys.error || !proof || !trust || !contextMatches ? (
                <Alert severity="error">{t("ballotPrivacy.proofError")}</Alert>
            ) : (
                <BallotLogProof trust={trust} proof={proof} />
            )}
            <Button
                disabled={loading}
                onClick={() => {
                    void entry.refetch().catch(() => undefined)
                    void keys.refetch().catch(() => undefined)
                }}
            >
                {t("ballotPrivacy.retry")}
            </Button>
        </Stack>
    )
}
