// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getGraphQLActionErrorMessage, getGraphQLActionErrorReason} from "./graphqlActionError"

describe("getGraphQLActionErrorReason", () => {
    it("reads the Harvest message Hasura forwarded", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [{message: "Can't edit a voter that has already cast its ballot"}],
            })
        ).toBe("Can't edit a voter that has already cast its ballot")
    })

    it("prefers the Harvest message in the webhook response body", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [
                    {
                        message: "unexpected response from webhook",
                        extensions: {
                            code: "unexpected",
                            internal: {
                                response: {
                                    body: JSON.stringify({
                                        message: "Cannot change tenant-id attribute",
                                        extensions: {code: "UnknownError"},
                                    }),
                                },
                            },
                        },
                    },
                ],
            })
        ).toBe("Cannot change tenant-id attribute")
    })

    it("falls back to the GraphQL message when the response body is not JSON", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [
                    {
                        message: "unexpected response from webhook",
                        extensions: {internal: {response: {body: "<html>502</html>"}}},
                    },
                ],
            })
        ).toBe("unexpected response from webhook")
    })

    it("falls back to the GraphQL message when the response body carries no message", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [
                    {
                        message: "unexpected response from webhook",
                        extensions: {
                            internal: {response: {body: JSON.stringify({error: "nope"})}},
                        },
                    },
                ],
            })
        ).toBe("unexpected response from webhook")
    })

    it("reports the transport failure when there is no response at all", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [
                    {
                        message: "unexpected",
                        extensions: {
                            code: "unexpected",
                            internal: {error: {message: "Response timeout"}},
                        },
                    },
                ],
            })
        ).toBe("Response timeout")
    })

    it("skips a GraphQL error that says nothing and reads the next one", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [{message: "   "}, {message: "Unauthorized"}],
            })
        ).toBe("Unauthorized")
    })

    it("keeps only the first line of a multi-line rejection", () => {
        expect(
            getGraphQLActionErrorReason({
                graphQLErrors: [{message: "Error editing user in Keycloak\n  at line 1\n  at 2"}],
            })
        ).toBe("Error editing user in Keycloak")
    })

    it("bounds a rejection long enough to overwhelm the form", () => {
        const reason = getGraphQLActionErrorReason({
            graphQLErrors: [{message: "x".repeat(5000)}],
        })

        expect(reason).toHaveLength(203)
        expect(reason?.endsWith("...")).toBe(true)
    })

    it("falls back to the network error message", () => {
        expect(getGraphQLActionErrorReason(new Error("Failed to fetch"))).toBe("Failed to fetch")
    })

    it("returns undefined when the error says nothing at all", () => {
        expect(getGraphQLActionErrorReason({graphQLErrors: []})).toBeUndefined()
        expect(getGraphQLActionErrorReason(undefined)).toBeUndefined()
    })
})

describe("getGraphQLActionErrorMessage", () => {
    it("preserves every line from a structured Harvest validation error", () => {
        const message = [
            "Ballot publication validation failed:",
            "- Contest A changed after voting started.",
            "- Contest B changed after voting started.",
        ].join("\n")

        expect(
            getGraphQLActionErrorMessage({
                graphQLErrors: [
                    {
                        message: "unexpected response from webhook",
                        extensions: {
                            internal: {
                                response: {
                                    body: JSON.stringify({
                                        message,
                                        extensions: {code: "BallotPublicationValidation"},
                                    }),
                                },
                            },
                        },
                    },
                ],
            })
        ).toBe(message)
    })

    it("bounds a multiline action response", () => {
        const message = getGraphQLActionErrorMessage({
            graphQLErrors: [{message: "x".repeat(5000)}],
        })

        expect(message).toHaveLength(4003)
        expect(message?.endsWith("...")).toBe(true)
    })
})
