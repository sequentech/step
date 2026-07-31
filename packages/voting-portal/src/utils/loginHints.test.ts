// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    InvalidLoginHintsError,
    MAX_LOGIN_HINT_COUNT,
    MAX_LOGIN_HINT_NAME_LENGTH,
    MAX_LOGIN_HINT_VALUE_LENGTH,
    appendLoginHints,
    parseLoginHints,
    removeLoginHintsFromSearch,
    routeAcceptsLoginHints,
} from "./loginHints"

describe("parseLoginHints", () => {
    it("returns no hints and preserves unrelated query parameters", () => {
        expect(parseLoginHints("?lang=en&kiosk")).toEqual({
            hints: {},
            remainingSearch: "?lang=en&kiosk=",
        })
    })

    it("extracts namespaced fields and removes only those fields", () => {
        expect(
            parseLoginHints(
                "?lang=es&login_hint__username=foo%2Bbar%40example.com&login_hint__dateOfBirth=2000-01-01"
            )
        ).toEqual({
            hints: {
                username: "foo+bar@example.com",
                dateOfBirth: "2000-01-01",
            },
            remainingSearch: "?lang=es",
        })
    })

    it("accepts the maximum number and size of fields", () => {
        const parameters = new URLSearchParams()
        const name = "a".repeat(MAX_LOGIN_HINT_NAME_LENGTH)
        parameters.set(`login_hint__${name}`, "v".repeat(MAX_LOGIN_HINT_VALUE_LENGTH))
        for (let index = 1; index < MAX_LOGIN_HINT_COUNT; index += 1) {
            parameters.set(`login_hint__field${index}`, `value${index}`)
        }

        expect(Object.keys(parseLoginHints(`?${parameters}`).hints)).toHaveLength(
            MAX_LOGIN_HINT_COUNT
        )
    })

    it.each([
        ["an empty field name", "?login_hint__=value"],
        ["an empty value", "?login_hint__username="],
        ["an invalid field name", "?login_hint__first%20name=value"],
        [
            "an overlong field name",
            `?login_hint__${"a".repeat(MAX_LOGIN_HINT_NAME_LENGTH + 1)}=value`,
        ],
        [
            "an overlong value",
            `?login_hint__username=${"a".repeat(MAX_LOGIN_HINT_VALUE_LENGTH + 1)}`,
        ],
        [
            "too many fields",
            `?${Array.from(
                {length: MAX_LOGIN_HINT_COUNT + 1},
                (_, index) => `login_hint__field${index}=value${index}`
            ).join("&")}`,
        ],
        ["a duplicate field", "?login_hint__username=first&login_hint__username=second"],
        ["an invalid percent escape", "?login_hint__username=user%ZZexample"],
        ["an invalid encoded hint name", "?login%5fhint__user%ZZ=secret"],
        ["invalid UTF-8", "?login_hint__username=%E0%A4%A"],
    ])("rejects %s", (_description, search) => {
        expect(() => parseLoginHints(search)).toThrow(InvalidLoginHintsError)
    })

    it("does not apply hint encoding validation to unrelated parameters", () => {
        expect(parseLoginHints("?return_to=%ZZ")).toEqual({
            hints: {},
            remainingSearch: "?return_to=%25ZZ",
        })
    })

    it("treats prototype property names as ordinary bounded hints", () => {
        const parsed = parseLoginHints("?login_hint____proto__=value&lang=en")

        expect(Object.hasOwn(parsed.hints, "__proto__")).toBe(true)
        expect(parsed.hints.__proto__).toBe("value")
        expect(parsed.remainingSearch).toBe("?lang=en")
    })

    it("removes invalid raw hint components while preserving unrelated query text", () => {
        expect(
            removeLoginHintsFromSearch(
                "?lang=en&login_hint__username=%E0%A4%A&login%5fhint__user%ZZ=secret&login_hint__username=duplicate&kiosk"
            )
        ).toBe("?lang=en&kiosk")
    })
})

describe("appendLoginHints", () => {
    it("preserves OIDC parameters and percent-encodes hint names and values", () => {
        const result = appendLoginHints(
            "https://id.example/authorize?state=oidc-state&nonce=oidc-nonce&code_challenge=pkce-challenge&redirect_uri=https%3A%2F%2Fvote.example%2Flogin&login_hint=user%40example.com",
            {
                username: "user@example.com",
                reference: "a&b=c % value",
            }
        )
        const resultUrl = new URL(result)

        expect(resultUrl.searchParams.get("state")).toBe("oidc-state")
        expect(resultUrl.searchParams.get("nonce")).toBe("oidc-nonce")
        expect(resultUrl.searchParams.get("code_challenge")).toBe("pkce-challenge")
        expect(resultUrl.searchParams.get("redirect_uri")).toBe("https://vote.example/login")
        expect(resultUrl.searchParams.get("login_hint")).toBe("user@example.com")
        expect(resultUrl.searchParams.get("login_hint__username")).toBe("user@example.com")
        expect(resultUrl.searchParams.get("login_hint__reference")).toBe("a&b=c % value")
        expect(resultUrl.searchParams.get("b")).toBeNull()
    })
})

describe("routeAcceptsLoginHints", () => {
    it.each(["/tenant/t/event/e/login", "/tenant/t/event/e/login/", "/enroll", "/enroll/"])(
        "accepts %s",
        (pathname) => {
            expect(routeAcceptsLoginHints(pathname)).toBe(true)
        }
    )

    it("rejects non-authentication routes", () => {
        expect(routeAcceptsLoginHints("/tenant/t/event/e/vote")).toBe(false)
    })
})
