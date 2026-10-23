// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TVitrinePure} from "./sdk"

const HEX_DIGEST = /^[0-9a-f]{64}$/

// Election public keys are base64 without padding; Vitrine's decoder takes
// canonical padded base64.
const padBase64 = (value: string): string => value + "=".repeat((4 - (value.length % 4)) % 4)

/** The election public key's bytes, decoded from the ballot style the booth
 * encrypts to. */
export const electionKeyBytes = (
    pure: TVitrinePure,
    publicKey: string | undefined
): Uint8Array | undefined => {
    if (!publicKey) return undefined
    try {
        return pure.encoding.b64decode(padBase64(publicKey))
    } catch {
        return undefined
    }
}

/** The config statement's `key` for that key: base64 of its SHA-256. */
export const electionKeyDigest = (
    pure: TVitrinePure,
    publicKey: string | undefined
): string | undefined => {
    const key = electionKeyBytes(pure, publicKey)
    return key && pure.encoding.b64encode(pure.hashes.sha256(key))
}

/** Whether `style_hashes` is as the contract serves it: lowercase hex
 * SHA-256 digests in ascending order, without repeats. Lowercase hex sorts
 * as the bytes do. */
export const isAscendingDigestList = (hexDigests: readonly string[]): boolean =>
    hexDigests.length > 0 &&
    hexDigests.every((h, i) => HEX_DIGEST.test(h) && (i === 0 || hexDigests[i - 1] < h))
