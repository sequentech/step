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
import type {Policy} from "@vitrine/verify"
import {
    BallotLogProof,
    type BallotLogEvidence,
    type BallotLogTrust,
} from "@sequentech/ui-essentials"
import {BALLOT_BOX_ENTRY, BALLOT_BOX_KEYS} from "../queries/BallotBoxLog"
import {shortBallotId} from "../services/vitrine/ballotId"
import {EVitrineLog} from "../services/vitrine/types"
import {explorersOf} from "./Vitrine/explorers"
import {FullTrackingId} from "./Vitrine/FullTrackingId"
import {EFindWhat, FindElsewhere} from "./Vitrine/FindElsewhere"

export interface BallotBoxEntryProps {
    electionEventId: string
    electionId: string
    ballotId: string
    /** The event's Vitrine policy, which names the explorers of the entry. */
    policy?: Policy
    onClose: () => void
}

export const BallotBoxEntry: React.FC<BallotBoxEntryProps> = ({
    electionEventId,
    electionId,
    ballotId,
    policy,
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
        proof?.tracking_id === ballotId &&
        trust?.election_id === electionId &&
        trust?.election_event_id === electionEventId
    const loadError = entry.error || keys.error || !proof || !trust || !contextMatches
    return (
        <Dialog
            className="ballot-proof-dialog"
            open
            onClose={onClose}
            fullWidth
            maxWidth="md"
            fullScreen={narrow}
            slotProps={{
                paper: {
                    sx: {maxWidth: 752, boxSizing: "border-box", borderRadius: narrow ? 0 : "4px"},
                },
            }}
            aria-labelledby="ballot-proof-title"
        >
            <DialogTitle
                className="ballot-proof-title"
                id="ballot-proof-title"
                sx={{fontSize: 16, fontWeight: 700, bgcolor: "#F7F9FC", py: 1.5}}
            >
                {t("ballotPrivacy.detailTitle", {ballotId: shortBallotId(ballotId)})}
            </DialogTitle>
            <DialogContent
                className="ballot-proof-content"
                sx={{padding: {xs: "16px 16px 0 !important", sm: "16px 48px 0 !important"}}}
            >
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
                    <>
                        <BallotLogProof
                            proof={proof}
                            trust={trust}
                            onRetry={retry}
                            renderTrackingId={(id) => <FullTrackingId ballotId={id} />}
                        />
                        <FindElsewhere
                            what={EFindWhat.ENTRY}
                            explorers={explorersOf(policy)}
                            policyId={policy?.id}
                            log={EVitrineLog.BALLOTS}
                            ballotId={ballotId}
                        />
                    </>
                )}
            </DialogContent>
            <DialogActions
                className="ballot-proof-actions"
                sx={{flexWrap: "wrap", px: {xs: 2, sm: 6}, pb: 2}}
            >
                {!loading && loadError ? (
                    <Button className="ballot-proof-retry-button" onClick={retry}>
                        {t("ballotPrivacy.retry")}
                    </Button>
                ) : null}
                <Button
                    className="ballot-proof-close-button"
                    variant="contained"
                    fullWidth
                    onClick={onClose}
                >
                    {t("ballotPrivacy.close")}
                </Button>
            </DialogActions>
        </Dialog>
    )
}
