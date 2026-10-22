// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {describe, expect, it} from "vitest"
import {LoginRestart, loginRestartOf, loginTimeoutLimitMs, sameMessage} from "./loginTimeout"

const REALM = "/realms/event"

describe("loginTimeoutLimitMs", () => {
    it("uses the login timeout, less a minute, on login pages", () => {
        const settings = {loginTimeoutSeconds: "1800", loginActionTimeoutSeconds: "300"}

        expect(loginTimeoutLimitMs(`${REALM}/protocol/openid-connect/auth`, settings)).toBe(
            1740 * 1000
        )
        expect(loginTimeoutLimitMs(`${REALM}/login-actions/authenticate`, settings)).toBe(
            1740 * 1000
        )
        expect(loginTimeoutLimitMs(`${REALM}/login-actions/registration`, settings)).toBe(
            1740 * 1000
        )
    })

    it("uses the login action timeout on required actions and password resets", () => {
        const settings = {loginTimeoutSeconds: "1800", loginActionTimeoutSeconds: "300"}

        expect(loginTimeoutLimitMs(`${REALM}/login-actions/required-action`, settings)).toBe(
            240 * 1000
        )
        expect(loginTimeoutLimitMs(`${REALM}/login-actions/reset-credentials`, settings)).toBe(
            240 * 1000
        )
    })

    it("falls back to Keycloak's defaults when the theme sets nothing usable", () => {
        expect(loginTimeoutLimitMs(`${REALM}/login-actions/authenticate`, {})).toBe(1740 * 1000)
        expect(
            loginTimeoutLimitMs(`${REALM}/login-actions/required-action`, {
                loginActionTimeoutSeconds: "${env.MISSING}",
            })
        ).toBe(240 * 1000)
    })

    it("restarts short timeouts halfway, before they expire", () => {
        expect(
            loginTimeoutLimitMs(`${REALM}/login-actions/authenticate`, {loginTimeoutSeconds: "60"})
        ).toBe(30 * 1000)
        expect(
            loginTimeoutLimitMs(`${REALM}/login-actions/authenticate`, {loginTimeoutSeconds: "30"})
        ).toBe(15 * 1000)
    })
})

describe("loginRestartOf", () => {
    it("reloads the authorization request", () => {
        expect(loginRestartOf(`${REALM}/protocol/openid-connect/auth`)).toBe(LoginRestart.Reload)
    })

    it("restarts the flow from a form result, which a reload would resubmit", () => {
        expect(loginRestartOf(`${REALM}/login-actions/authenticate`)).toBe(LoginRestart.RestartFlow)
        expect(loginRestartOf(`${REALM}/login-actions/required-action`)).toBe(
            LoginRestart.RestartFlow
        )
    })
})

describe("sameMessage", () => {
    it("matches the server's notice against Keycloakify's bundled text", () => {
        expect(
            sameMessage(
                "Your login attempt timed out. Login will start from the beginning.",
                "Your login attempt timed out.  Login will start from the beginning."
            )
        ).toBe(true)
        expect(sameMessage("Invalid username or password.", "Your login attempt timed out.")).toBe(
            false
        )
    })
})
