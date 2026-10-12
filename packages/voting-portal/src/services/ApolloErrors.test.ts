// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {CombinedGraphQLErrors, ServerError} from "@apollo/client/errors"
import {isApolloTransportError} from "./ApolloErrors"

test("recognizes Apollo 4 HTTP failures and rejected fetches", () => {
    expect(
        isApolloTransportError(
            new ServerError("Service unavailable", {
                response: new Response("", {status: 503}),
                bodyText: "",
            })
        )
    ).toBe(true)
    expect(isApolloTransportError(new TypeError("Failed to fetch"))).toBe(true)
})

test("GraphQL authorization failures are not misreported as connectivity errors", () => {
    expect(
        isApolloTransportError(new CombinedGraphQLErrors({errors: [{message: "Not authorized"}]}))
    ).toBe(false)
    expect(isApolloTransportError(undefined)).toBe(false)
})
