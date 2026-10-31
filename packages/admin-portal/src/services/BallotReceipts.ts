// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EReceiptsPolicy, EVotingStatus} from "@sequentech/ui-core"
import type {IElectionEventPresentation, IElectionEventStatus} from "@sequentech/ui-core"

export const RECEIPTS_POLICY_SOURCE = "presentation.receipts.policy"
export const VOTER_SIGNING_POLICY_SOURCE = "presentation.voter_signing_policy"

export const areReceiptsSignedByBallotBox = (
    presentation?: Pick<IElectionEventPresentation, "receipts">
): boolean => presentation?.receipts?.policy === EReceiptsPolicy.SIGNED_BY_BALLOT_BOX

// Ballots already received or cast were made under the published setting.
export const hasVotingStarted = (status?: Partial<IElectionEventStatus> | null): boolean =>
    [
        status?.voting_status,
        status?.kiosk_voting_status,
        status?.early_voting_status,
        status?.telephone_voting_status,
    ].some((channel) => channel !== undefined && channel !== EVotingStatus.NOT_STARTED)

export const receiptsHelperTextKey = (votingStarted: boolean): string =>
    votingStarted
        ? "electionEventScreen.field.receiptsPolicy.lockedHelperText"
        : "electionEventScreen.field.receiptsPolicy.helperText"

export const parseChecksAvailableUntil = (value?: string | null): string | null =>
    value && !Number.isNaN(new Date(value).getTime()) ? new Date(value).toISOString() : null
