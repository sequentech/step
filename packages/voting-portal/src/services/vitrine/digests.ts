// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import type {TVitrinePure} from "./sdk"

const SHA256_BYTES = 32

// Election public keys are base64 without padding; Vitrine's decoder takes
// canonical padded base64.
const padBase64 = (value: string): string => value + "=".repeat((4 - (value.length % 4)) % 4)

/** The config statement's `key`: base64 SHA-256 of the election public key's
 * bytes, decoded from the ballot style the booth encrypts to. */
export const electionKeyDigest = (
    pure: TVitrinePure,
    publicKey: string | undefined
): string | undefined => {
    if (!publicKey) return undefined
    try {
        const key = pure.encoding.b64decode(padBase64(publicKey))
        return pure.encoding.b64encode(pure.hashes.sha256(key))
    } catch {
        return undefined
    }
}

/** Hex SHA-256 of one ballot style's text, as `style_hashes` lists it. */
export const styleDigestHex = (
    pure: TVitrinePure,
    styleText: string | undefined
): string | undefined => {
    if (styleText === undefined) return undefined
    try {
        return pure.encoding.hexEncode(pure.hashes.sha256(styleText))
    } catch {
        return undefined
    }
}

/** The config statement's `styles`: SHA-256 over the concatenation of the
 * 32-byte digests of the election's ballot styles, as `style_hashes` lists
 * them. The list must be in ascending order without repeats (Vitrine's
 * specs/entry.md, section 5.2): any other list hashes to nothing. */
export const stylesDigest = (
    pure: TVitrinePure,
    hexDigests: readonly string[]
): string | undefined => {
    if (hexDigests.length === 0) return undefined
    try {
        const digests = hexDigests.map((hex) => pure.encoding.hexDecode(hex, SHA256_BYTES))
        for (let i = 1; i < digests.length; i++) {
            if (pure.bytes.compareBytes(digests[i - 1], digests[i]) >= 0) return undefined
        }
        return pure.encoding.b64encode(pure.hashes.sha256(...digests))
    } catch {
        return undefined
    }
}
