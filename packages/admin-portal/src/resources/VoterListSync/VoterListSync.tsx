// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useState} from "react"
import {Stack, Typography} from "@mui/material"
import {Tabs} from "@/components/Tabs"
import {IndeterminateVotesTab} from "./IndeterminateVotesTab"
import {GeneratePatchesWizard} from "./GeneratePatchesWizard"
import {ApplyPatchWizard} from "./ApplyPatchWizard"

/**
 * "VoterView Sync" election-event tab: hosts the indeterminate-ballot
 * resolution view and the two Datafix reconciliation wizards described in
 * DatafixPossibleImplementation.md#import-flows. All three are UI
 * prototypes - see mockSyncEngine.ts/mockIndeterminateEngine.ts for what
 * still needs to be rewired to the real backend (S3 upload, diff-calculation
 * task, generate/apply-patch Celery tasks, cast_vote resolution mutation).
 */
interface VoterListSyncProps {
    electionEventId?: string
}

export const VoterListSync: React.FC<VoterListSyncProps> = ({electionEventId}) => {
    // MOCK: the last applied reconciliation Sequence would normally be read
    // from the election event record (stale-file protection). Seeded at -1
    // so a fresh Sequence=0 kickoff file is accepted on first load.
    const [lastAppliedSequence, setLastAppliedSequence] = useState<number>(-1)
    // Shared with GeneratePatchesWizard so it can warn on import - see
    // "Indeterminate Ballot Resolution" in DatafixPossibleImplementation.md.
    const [pendingIndeterminateCount, setPendingIndeterminateCount] = useState<number>(0)
    const handlePendingCountChange = useCallback((count: number) => {
        setPendingIndeterminateCount(count)
    }, [])

    const tabs = [
        {
            label: "Indeterminate votes",
            component: IndeterminateVotesTab,
            props: {electionEventId, onPendingCountChange: handlePendingCountChange},
        },
        {
            label: "Generate patches",
            component: GeneratePatchesWizard,
            props: {lastAppliedSequence, pendingIndeterminateCount},
        },
        {
            label: "Apply patch",
            component: ApplyPatchWizard,
            props: {lastAppliedSequence, onApplied: setLastAppliedSequence},
        },
    ]

    return (
        <Stack spacing={2}>
            <Typography variant="body2" color="text.secondary">
                Last applied reconciliation Sequence:{" "}
                {lastAppliedSequence < 0 ? "none yet" : lastAppliedSequence}
            </Typography>
            <Tabs elements={tabs} />
        </Stack>
    )
}

export default VoterListSync
