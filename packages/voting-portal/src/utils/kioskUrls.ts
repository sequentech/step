// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export const getKioskAwareUrl = (
    defaultUrl: string,
    kioskUrl: string | undefined,
    isKiosk: boolean
): string => (isKiosk && kioskUrl?.trim() ? kioskUrl.trim() : defaultUrl)

export const getKioskPortalRedirectUrl = (
    currentUrl: string,
    kioskPortalUrl: string | undefined,
    isKiosk: boolean
): string | null => {
    if (!isKiosk || !kioskPortalUrl?.trim()) {
        return null
    }

    const current = new URL(currentUrl)
    const target = new URL(kioskPortalUrl.trim())
    if (current.origin === target.origin) {
        return null
    }

    target.pathname = current.pathname
    target.search = current.search
    target.hash = current.hash
    return target.toString()
}
