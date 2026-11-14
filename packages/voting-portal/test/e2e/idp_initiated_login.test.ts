// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

const ssoUrl = process.env.IDP_INITIATED_SSO_URL
const username = process.env.IDP_INITIATED_USERNAME
const password = process.env.IDP_INITIATED_PASSWORD
const usernameSelector = process.env.IDP_INITIATED_USERNAME_SELECTOR ?? 'input[name="username"]'
const passwordSelector = process.env.IDP_INITIATED_PASSWORD_SELECTOR ?? 'input[name="password"]'
const required = process.env.IDP_INITIATED_LOGIN_REQUIRED === "true"
const identityProviderTimeoutMs = 30_000
const votingPortalTimeoutMs = 60_000

const configured = Boolean(ssoUrl && username && password)
if (!configured && required) {
    throw new Error(
        "IDP_INITIATED_SSO_URL, IDP_INITIATED_USERNAME and IDP_INITIATED_PASSWORD are required when IDP_INITIATED_LOGIN_REQUIRED=true"
    )
}

describe("login started at an external identity provider", function () {
    const test = configured ? it : it.skip

    test("lands on the election list with the event's page branding", function (browser: NightwatchAPI) {
        browser
            .navigateTo(ssoUrl!)
            .waitForElementVisible(usernameSelector, identityProviderTimeoutMs)
            .setValue(usernameSelector, username!)
            .setValue(passwordSelector, [password!, browser.Keys.ENTER])
            .waitForElementVisible(".election-selection-screen", votingPortalTimeoutMs)
            .assert.urlContains("/election-chooser")
            .assert.not.elementPresent(".neutral-loader")
            .assert.elementPresent(".voting-portal-wrapper .election-selection-screen")
            .assert.elementPresent("link[data-page-branding]")
    })

    after(function (browser) {
        browser.end()
    })
})
