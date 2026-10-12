// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {EAllowTally, EVotingStatus, IElectionStatus} from "@sequentech/ui-core"

type TallyElection = {
    id: string
    status?: (Partial<IElectionStatus> & {allow_tally?: EAllowTally}) | null
}

export const getTallyDisabledReason = (
    elections: TallyElection[] | undefined,
    selectedIds: string[] | undefined
): string | undefined => {
    if (!elections || !selectedIds?.length) return "selectElection"
    for (const id of selectedIds) {
        const election = elections.find((item) => item.id === id)
        if (!election?.status?.is_published) return "publishElection"
        const status = election.status
        const policy = status.allow_tally ?? EAllowTally.ALLOWED
        if (policy === EAllowTally.ALLOWED) continue
        if (policy !== EAllowTally.REQUIRES_VOTING_PERIOD_END) return "tallyDisallowed"
        const secondaryClosed = [
            status.kiosk_voting_status,
            status.early_voting_status,
            status.telephone_voting_status,
        ].every(
            (value) =>
                !value || value === EVotingStatus.NOT_STARTED || value === EVotingStatus.CLOSED
        )
        if (status.voting_status !== EVotingStatus.CLOSED || !secondaryClosed) return "endVoting"
    }
    return undefined
}
