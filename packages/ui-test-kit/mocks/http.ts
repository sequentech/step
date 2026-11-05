// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/** A browser request as the mocks see it, independent of the test runner. */
export interface MockRequest {
    method: string
    url: URL
    /** Header names are lower case. */
    headers: Record<string, string>
    body?: string
}

/** Browser network error codes a mock may answer with instead of a response. */
export type MockAbortReason =
    | "aborted"
    | "connectionclosed"
    | "connectionfailed"
    | "connectionrefused"
    | "failed"
    | "internetdisconnected"
    | "timedout"

export interface MockFulfillment {
    status: number
    headers?: Record<string, string>
    body?: string | Uint8Array
}

export type MockResponse = MockFulfillment | {abort: MockAbortReason}

export const isAbort = (response: MockResponse): response is {abort: MockAbortReason} =>
    "abort" in response

export const json = (
    status: number,
    value: unknown,
    headers: Record<string, string> = {}
): MockFulfillment => ({
    status,
    headers: {"content-type": "application/json; charset=utf-8", ...headers},
    body: JSON.stringify(value),
})

export const html = (status: number, body: string): MockFulfillment => ({
    status,
    headers: {"content-type": "text/html; charset=utf-8"},
    body,
})

export const redirect = (location: string): MockFulfillment => ({
    status: 302,
    headers: {"location": location, "cache-control": "no-store"},
})

export const escapeHtml = (value: string): string =>
    value.replace(
        /[&<>"']/g,
        (character) =>
            ({"&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"})[character] ??
            character
    )

/** Same-origin paths where the Playwright adapter serves each mock. */
export const MOCK_PATHS = {
    keycloak: "/keycloak",
    graphql: "/v1/graphql",
    s3: "/s3",
} as const
