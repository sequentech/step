// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

const username = "prefilled-voter@example.com"
const dateOfBirth = "2000-01-01"
const dateOfBirthSelector = 'input[name="dateOfBirth"]'
const browserMatrixRequired = process.env.PREFILL_BROWSER_MATRIX_REQUIRED === "true"
const authenticationPageTimeoutMs = 30_000
const preservedLanguage = "es"
const preservedKiosk = "true"

type HintScenario = {
    environmentVariable: string
    route: "login" | "enroll"
    fieldSelector: string
    expectedValue: string
    hints: Record<string, string>
}

const scenarios: Record<string, HintScenario> = {
    "stock username login": {
        environmentVariable: "PREFILL_STOCK_LOGIN_URL",
        route: "login",
        fieldSelector: 'input[name="username"]',
        expectedValue: username,
        hints: {username},
    },
    "direct stock registration": {
        environmentVariable: "PREFILL_STOCK_REGISTRATION_URL",
        route: "enroll",
        fieldSelector: dateOfBirthSelector,
        expectedValue: dateOfBirth,
        hints: {username, dateOfBirth},
    },
    "login redirected to stock registration": {
        environmentVariable: "PREFILL_REDIRECT_REGISTRATION_URL",
        route: "login",
        fieldSelector: dateOfBirthSelector,
        expectedValue: dateOfBirth,
        hints: {username, dateOfBirth},
    },
    "deferred registration ignores hints by default": {
        environmentVariable: "PREFILL_DEFERRED_IGNORE_URL",
        route: "enroll",
        fieldSelector: dateOfBirthSelector,
        expectedValue: "",
        hints: {dateOfBirth},
    },
    "deferred registration accepts hints when enabled": {
        environmentVariable: "PREFILL_DEFERRED_ACCEPT_URL",
        route: "enroll",
        fieldSelector: dateOfBirthSelector,
        expectedValue: dateOfBirth,
        hints: {dateOfBirth},
    },
}

function buildVotingPortalUrl(scenario: HintScenario): string | undefined {
    const configuredUrl = process.env[scenario.environmentVariable]
    if (!configuredUrl) {
        if (browserMatrixRequired) {
            throw new Error(
                `${scenario.environmentVariable} is required when PREFILL_BROWSER_MATRIX_REQUIRED=true`
            )
        }
        return undefined
    }

    const url = new URL(configuredUrl)
    const normalizedPath = url.pathname.replace(/\/$/, "")
    if (!normalizedPath.endsWith(`/${scenario.route}`)) {
        throw new Error(
            `${scenario.environmentVariable} must be a Voting Portal /${scenario.route} URL`
        )
    }

    url.searchParams.set("lang", preservedLanguage)
    url.searchParams.set("kiosk", preservedKiosk)
    for (const [fieldName, value] of Object.entries(scenario.hints)) {
        url.searchParams.set(`login_hint__${fieldName}`, value)
    }
    return url.toString()
}

function assertHintFreeReturnUrl(browser: NightwatchAPI): void {
    browser.execute(
        function () {
            const authorizationUrl = new URL(window.location.href)
            const rawReturnUrl = authorizationUrl.searchParams.get("redirect_uri")
            if (!rawReturnUrl) {
                return {error: "missing redirect_uri"}
            }

            const returnUrl = new URL(rawReturnUrl)
            return {
                hasLoginHints: Array.from(returnUrl.searchParams.keys()).some((name) =>
                    name.startsWith("login_hint__")
                ),
                kiosk: returnUrl.searchParams.get("kiosk"),
                lang: returnUrl.searchParams.get("lang"),
            }
        },
        [],
        (result) => {
            browser.assert.deepEqual(result.value, {
                hasLoginHints: false,
                kiosk: preservedKiosk,
                lang: preservedLanguage,
            })
        }
    )
}

describe("login hint browser matrix", function () {
    for (const [name, scenario] of Object.entries(scenarios)) {
        const votingPortalUrl = buildVotingPortalUrl(scenario)
        const test = votingPortalUrl ? it : it.skip

        test(name, function (browser: NightwatchAPI) {
            browser
                .navigateTo(votingPortalUrl!)
                .waitForElementVisible("body", authenticationPageTimeoutMs)
                .waitForElementVisible(scenario.fieldSelector, authenticationPageTimeoutMs)
                .assert.valueEquals(scenario.fieldSelector, scenario.expectedValue)
            assertHintFreeReturnUrl(browser)
        })
    }

    after(function (browser) {
        browser.end()
    })
})
