// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The system hash as the voter apps show it: the first eight hex digits of the deployed
 * revision, in upper case, as "DEAD-BEEF". Undefined for a deployment without a real hash,
 * such as "-" or "dev".
 */
export const shortAppHash = (appHash?: string): string | undefined => {
    if (!appHash || !/^[0-9a-f]{8,}$/i.test(appHash)) {
        return undefined
    }
    const digits = appHash.slice(0, 8).toUpperCase()
    return `${digits.slice(0, 4)}-${digits.slice(4)}`
}
