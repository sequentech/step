// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITallyCeremonyStatus, ITallyTrusteeStatus} from "@/types/ceremonies"

interface TallyCeremonyExecutionLike {
    status?: ITallyCeremonyStatus | null
}

export const getTallyTrusteeStatus = (
    execution: TallyCeremonyExecutionLike | undefined,
    trusteeName: string | null | undefined
): ITallyTrusteeStatus | null => {
    if (!trusteeName) {
        return null
    }
    return (
        execution?.status?.trustees.find((trustee) => trustee.name === trusteeName)?.status ?? null
    )
}
