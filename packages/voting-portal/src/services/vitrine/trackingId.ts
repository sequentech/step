// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Tracking ID of a ballot: what the voter reads (52 characters in groups
// of four) and, decoded, the `ballot_id` of every entry the ballot gets in the
// ballots log (Vitrine's specs/receipt.md, section 1.1).

import {trackingId} from "@sequentech/ui-core"
import {loadVitrineSdk} from "./sdk"

/** The Tracking ID of the ballot with this hash, signed by this voter key. The
 * ballot box returns nothing that goes into it, so it is known before the
 * ballot is sent. Undefined when it cannot be computed. */
export const trackingIdOf = (
    ballotHash: string | undefined,
    voterSigningPk: string | undefined | null
): string | undefined => {
    if (!ballotHash || !voterSigningPk) return undefined
    try {
        return trackingId(ballotHash, voterSigningPk)
    } catch {
        return undefined
    }
}

/** The Tracking ID's digest as 64 lowercase hex digits. Undefined when `id` is
 * not a full Tracking ID. */
export const trackingDigestHex = async (id: string): Promise<string | undefined> => {
    try {
        const sdk = await loadVitrineSdk()
        return sdk.pure.encoding.hexEncode(sdk.verify.decodeTrackingId(id))
    } catch {
        return undefined
    }
}
