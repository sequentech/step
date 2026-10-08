// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The `seal-at-close` value of `EBallotBoxSealPolicy`, without importing ui-core. */
const SEAL_AT_CLOSE = "seal-at-close"

/**
 * Whether an election event seals its ballot boxes at close, from its
 * presentation (an object, or its JSON text). Any other or no value means it
 * doesn't, and the portal then reads no seals at all.
 */
export const isSealAtClose = (presentation: unknown): boolean => {
    let value = presentation
    if (typeof value === "string") {
        try {
            value = JSON.parse(value)
        } catch {
            return false
        }
    }
    return (
        !!value &&
        typeof value === "object" &&
        (value as {ballot_box_seal_policy?: unknown}).ballot_box_seal_policy === SEAL_AT_CLOSE
    )
}
