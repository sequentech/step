// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {
    KEPT_FIELD_NAMES,
    LoginRestart,
    loginRestartOf,
    loginTimeoutUrls,
    sameMessage,
} from "./loginTimeout"

const BASE = "https://auth.example.com/realms/event/login-actions/authenticate"
const RESTART = "/realms/event/login-actions/restart?skip_logout=true"
const LOGIN_ACTION =
    "/realms/event/login-actions/authenticate?session_code=s&execution=e&client_id=portal&tab_id=tab-1"

describe("loginTimeoutUrls", () => {
    it("asks the realm about the tab the form submits to", () => {
        expect(loginTimeoutUrls(RESTART, LOGIN_ACTION, BASE)?.status.href).toBe(
            "https://auth.example.com/realms/event/login-timeout?tab_id=tab-1"
        )
    })

    it("restarts that tab's login without signing out", () => {
        expect(loginTimeoutUrls(RESTART, LOGIN_ACTION, BASE)?.restart.href).toBe(
            "https://auth.example.com/realms/event/login-actions/restart?skip_logout=true&client_id=portal&tab_id=tab-1"
        )
    })

    it("asks about the whole login when the form names no tab", () => {
        expect(loginTimeoutUrls(RESTART, "", BASE)?.status.href).toBe(
            "https://auth.example.com/realms/event/login-timeout"
        )
    })

    it("stays on Keycloak's origin", () => {
        const urls = loginTimeoutUrls(RESTART, "https://other.example.com/?tab_id=x", BASE)

        expect(urls?.status.origin).toBe("https://auth.example.com")
        expect(urls?.restart.origin).toBe("https://auth.example.com")
    })

    it("has nothing to ask outside Keycloak's login actions", () => {
        expect(loginTimeoutUrls("#", LOGIN_ACTION, "http://localhost:6006/iframe.html")).toBeNull()
    })
})

describe("loginRestartOf", () => {
    it("reloads the first page of the flow", () => {
        expect(loginRestartOf("/realms/event/protocol/openid-connect/auth")).toBe(
            LoginRestart.Reload
        )
    })

    it("restarts the flow from any later page", () => {
        expect(loginRestartOf("/realms/event/login-actions/authenticate")).toBe(
            LoginRestart.RestartFlow
        )
        expect(loginRestartOf("/realms/event/login-actions/required-action")).toBe(
            LoginRestart.RestartFlow
        )
    })
})

describe("KEPT_FIELD_NAMES", () => {
    it("keeps only the fields that identify the voter", () => {
        expect(KEPT_FIELD_NAMES).toEqual(["username", "email"])
    })
})

describe("sameMessage", () => {
    it("ignores whitespace differences", () => {
        expect(
            sameMessage(
                "Your login attempt timed out.  Login will start from the beginning.",
                "Your login attempt timed out. Login will start from the beginning."
            )
        ).toBe(true)
        expect(sameMessage("Invalid username or password.", "Your login attempt timed out.")).toBe(
            false
        )
    })
})
