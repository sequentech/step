// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {ETallyProvenance, TALLY_PROVENANCE_ANNOTATION_KEY} from "@/types/ceremonies"

export const getTallyProvenance = (annotations: unknown): ETallyProvenance => {
    if (annotations === null || typeof annotations !== "object") {
        return ETallyProvenance.NATIVE
    }
    const value = (annotations as Record<string, unknown>)[TALLY_PROVENANCE_ANNOTATION_KEY]
    return value === ETallyProvenance.IMPORTED ? ETallyProvenance.IMPORTED : ETallyProvenance.NATIVE
}
