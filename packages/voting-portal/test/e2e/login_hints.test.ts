// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

const username = "prefilled-voter@example.com"
const dateOfBirth = "2000-01-01"
const dateOfBirthSelector = 'input[name="dateOfBirth"]'

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
        return undefined
    }

    const url = new URL(configuredUrl)
    if (!url.pathname.endsWith(`/${scenario.route}`)) {
        throw new Error(
            `${scenario.environmentVariable} must be a Voting Portal /${scenario.route} URL`
        )
    }

    for (const [fieldName, value] of Object.entries(scenario.hints)) {
        url.searchParams.set(`login_hint__${fieldName}`, value)
    }
    return url.toString()
}

describe("login hint browser matrix", function () {
    for (const [name, scenario] of Object.entries(scenarios)) {
        const votingPortalUrl = buildVotingPortalUrl(scenario)
        const test = votingPortalUrl ? it : it.skip

        test(name, function (browser: NightwatchAPI) {
            browser
                .navigateTo(votingPortalUrl!)
                .waitForElementVisible("body")
                .waitForElementVisible(scenario.fieldSelector)
                .assert.valueEquals(scenario.fieldSelector, scenario.expectedValue)
        })
    }

    after(function (browser) {
        browser.end()
    })
})
