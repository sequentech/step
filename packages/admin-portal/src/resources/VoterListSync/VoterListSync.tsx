// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {Stack, Typography} from "@mui/material"
import {Tabs} from "@/components/Tabs"
import {GeneratePatchesWizard} from "./GeneratePatchesWizard"
import {ApplyPatchWizard} from "./ApplyPatchWizard"

/**
 * "Voter List Sync" election-event tab: hosts the two Datafix reconciliation
 * wizards described in DatafixPossibleImplementation.md#import-flows. Both
 * wizards are UI prototypes - see mockSyncEngine.ts for what still needs to
 * be rewired to the real backend (S3 upload, diff-calculation task,
 * generate/apply-patch Celery tasks).
 */
export const VoterListSync: React.FC = () => {
    // MOCK: the last applied reconciliation Sequence would normally be read
    // from the election event record (stale-file protection). Seeded at -1
    // so a fresh Sequence=0 kickoff file is accepted on first load.
    const [lastAppliedSequence, setLastAppliedSequence] = useState<number>(-1)

    const tabs = [
        {
            label: "Generate patches",
            component: GeneratePatchesWizard,
            props: {lastAppliedSequence},
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
