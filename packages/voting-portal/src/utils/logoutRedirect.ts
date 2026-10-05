// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

interface LogoutRedirectUrlOptions {
    redirectUrl?: string
    origin: string
    pathname: string
    tenantId: string | null
    eventId: string | null
    clientId?: string
    defaultClientId: string
    hasKioskQuery: boolean
}

export const isKioskClientId = (clientId: string | undefined, defaultClientId: string) =>
    clientId === `${defaultClientId}-kiosk`

export const getLogoutRedirectUrl = ({
    redirectUrl,
    origin,
    pathname,
    tenantId,
    eventId,
    clientId,
    defaultClientId,
    hasKioskQuery,
}: LogoutRedirectUrlOptions) => {
    if (redirectUrl) {
        return redirectUrl
    }

    const isKiosk = isKioskClientId(clientId, defaultClientId) || (!clientId && hasKioskQuery)
    if (isKiosk) {
        return `${origin}/tenant/${tenantId}/event/${eventId}/login?kiosk`
    }

    const pathSegments = pathname.split("/")
    while (pathSegments.length > 5) {
        pathSegments.pop()
    }
    return origin + pathSegments.join("/")
}
