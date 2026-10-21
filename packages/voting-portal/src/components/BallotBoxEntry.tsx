// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useQuery} from "@apollo/client/react"
import {
    Alert,
    Button,
    CircularProgress,
    Dialog,
    DialogActions,
    DialogContent,
    DialogTitle,
    useMediaQuery,
    useTheme,
} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    BallotLogProof,
    type BallotLogEvidence,
    type BallotLogTrust,
} from "@sequentech/ui-essentials"
import {BALLOT_BOX_ENTRY, BALLOT_BOX_KEYS} from "../queries/BallotBoxLog"

export interface BallotBoxEntryProps {
    electionEventId: string
    electionId: string
    ballotId: string
    onClose: () => void
}

export const BallotBoxEntry: React.FC<BallotBoxEntryProps> = ({
    electionEventId,
    electionId,
    ballotId,
    onClose,
}) => {
    const {t} = useTranslation()
    const theme = useTheme()
    const narrow = useMediaQuery(theme.breakpoints.down("sm"))
    const entry = useQuery<{ballot_box_entry: {payload: BallotLogEvidence}}>(BALLOT_BOX_ENTRY, {
        variables: {electionEventId, electionId, ballotId},
        fetchPolicy: "no-cache",
    })
    const keys = useQuery<{ballot_box_keys: {payload: BallotLogTrust}}>(BALLOT_BOX_KEYS, {
        variables: {electionEventId, electionId, ballotId},
        fetchPolicy: "no-cache",
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
        trust?.election_id === electionId &&
        trust?.election_event_id === electionEventId
    const loadError = entry.error || keys.error || !proof || !trust || !contextMatches
    return (
        <Dialog
            className="ballot-proof-dialog"
            open
            onClose={onClose}
            fullWidth
            maxWidth="sm"
            fullScreen={narrow}
            aria-labelledby="ballot-proof-title"
        >
            <DialogTitle className="ballot-proof-title" id="ballot-proof-title">
                {t("ballotPrivacy.detailTitle", {ballotId})}
            </DialogTitle>
            <DialogContent className="ballot-proof-content">
                {loading ? (
                    <CircularProgress
                        className="ballot-proof-loading"
                        aria-label={t("ballotPrivacy.checking")}
                    />
                ) : loadError ? (
                    <Alert className="ballot-proof-error" severity="error">
                        {t("ballotPrivacy.proofError")}
                    </Alert>
                ) : (
                    <BallotLogProof proof={proof} trust={trust} onRetry={retry} />
                )}
            </DialogContent>
            <DialogActions className="ballot-proof-actions" sx={{flexWrap: "wrap"}}>
                {!loading && loadError ? (
                    <Button className="ballot-proof-retry-button" onClick={retry}>
                        {t("ballotPrivacy.retry")}
                    </Button>
                ) : null}
                <Button className="ballot-proof-close-button" onClick={onClose}>
                    {t("ballotPrivacy.close")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
