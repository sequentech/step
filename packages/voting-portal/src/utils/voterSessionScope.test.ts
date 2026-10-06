// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {voterSessionScope} from "./voterSessionScope"

const token = (claims: object) =>
    `header.${btoa(JSON.stringify(claims)).replace(/=/g, "").replace(/\+/g, "-").replace(/\//g, "_")}.signature`
const claims = {
    "sub": "voter",
    "azp": "voting-portal",
    "exp": 100,
    "https://hasura.io/jwt/claims": {
        "x-hasura-tenant-id": "tenant",
        "x-hasura-election-event-id": "event",
        "x-hasura-area-id": "area",
        "authorized-election-ids": ["election"],
        "x-hasura-allowed-roles": ["user"],
    },
}

test("routine access-token refresh preserves the voter session partition", () => {
    expect(voterSessionScope(token(claims))).toBe(voterSessionScope(token({...claims, exp: 200})))
})

test.each([
    "x-hasura-tenant-id",
    "x-hasura-election-event-id",
    "x-hasura-area-id",
    "authorized-election-ids",
    "x-hasura-allowed-roles",
])("changing %s invalidates cached eligibility", (key) => {
    const changed = {
        ...claims,
        "https://hasura.io/jwt/claims": {
            ...claims["https://hasura.io/jwt/claims"],
            [key]: "changed",
        },
    }
    expect(voterSessionScope(token(claims))).not.toBe(voterSessionScope(token(changed)))
})

test("voter, client, logout and malformed token changes cannot reuse a session", () => {
    const original = voterSessionScope(token(claims))
    expect(original).not.toBe(voterSessionScope(token({...claims, sub: "another-voter"})))
    expect(original).not.toBe(voterSessionScope(token({...claims, azp: "voting-portal-kiosk"})))
    expect(original).not.toBe(voterSessionScope(token({...claims, acr: "gold"})))
    expect(original).not.toBe(voterSessionScope(token({...claims, sid: "new-session"})))
    expect(original).not.toBe(voterSessionScope())
    expect(original).not.toBe(voterSessionScope("malformed"))
})
