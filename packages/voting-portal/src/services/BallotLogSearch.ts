// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** Prefixes share the short-code alphabet; null must never become an empty search. */
export const normalizeBallotLogPrefix = (input: string): string | null => {
    const value = input.replace(/[-\s]/g, "").toUpperCase()
    return /^[0-9ABCDEFGHJKMNPQRSTVWXYZ]{0,8}$/.test(value) ? value : null
}
