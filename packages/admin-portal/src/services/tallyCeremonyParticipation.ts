// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITallyCeremonyStatus, ITallyExecutionStatus} from "@/types/ceremonies"

interface TallyCeremonyExecutionLike {
    status?: ITallyCeremonyStatus | null
}

export const isTrusteeInTallyCeremony = (
    execution: TallyCeremonyExecutionLike | undefined,
    trusteeName: string | null | undefined
): boolean =>
    !!trusteeName &&
    execution?.status?.trustees.some((trustee) => trustee.name === trusteeName) === true

export const isTallyAcceptingTrusteeKeys = (executionStatus: string | null | undefined): boolean =>
    executionStatus === ITallyExecutionStatus.STARTED ||
    executionStatus === ITallyExecutionStatus.CONNECTED
