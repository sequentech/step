// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {EAllowTally, EVotingStatus, IElectionStatus} from "@sequentech/ui-core"
import {IKeysCeremonyExecutionStatus} from "./KeyCeremony"

type TallyElection = {
    id: string
    keys_ceremony_id?: string | null
    status?: (Partial<IElectionStatus> & {allow_tally?: EAllowTally}) | null
}

type TallyKeysCeremony = {
    id: string
    execution_status?: string | null
}

export const getTallyDisabledReason = (
    elections: TallyElection[] | undefined,
    selectedIds: string[] | undefined,
    // Mirrors windmill's find_keys_ceremony. Undefined skips the check (still loading).
    keysCeremonies?: TallyKeysCeremony[]
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
    if (!keysCeremonies) return undefined
    const keysIds = selectedIds.map(
        (id) => elections.find((item) => item.id === id)?.keys_ceremony_id
    )
    // Same order as windmill: distinct assigned ceremonies first, then missing ones.
    if (new Set(keysIds.filter(Boolean)).size > 1) return "keysCeremonyMismatch"
    if (keysIds.some((id) => !id)) return "keysCeremonyMissing"
    const ceremony = keysCeremonies.find((item) => item.id === keysIds[0])
    if (ceremony?.execution_status !== IKeysCeremonyExecutionStatus.SUCCESS)
        return "keysCeremonyIncomplete"
    return undefined
}
