// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** A cache partition only; the server remains responsible for validating JWTs. */
export function voterSessionScope(token?: string): string | undefined {
    if (!token) return undefined
    try {
        const payload = token.split(".")[1].replace(/-/g, "+").replace(/_/g, "/")
        const claims = JSON.parse(atob(payload))
        return JSON.stringify({
            subject: claims.sub,
            client: claims.azp,
            session: claims.sid,
            assurance: claims.acr,
            authorization: claims["https://hasura.io/jwt/claims"],
        })
    } catch {
        // A malformed replacement must never share a previous session's cache.
        return token
    }
}
