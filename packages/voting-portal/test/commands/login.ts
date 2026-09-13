// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    admin_portal_password,
    admin_portal_username,
    loginUrl as testUrl,
    NightWatchLogin,
    pause,
} from "../index"

exports.command = function ({
    loginUrl = testUrl,
    username = admin_portal_username,
    password = admin_portal_password,
}): NightWatchLogin {
    const usernameSelector = "input[name=username]"
    this.username = usernameSelector
    const passwordSelector = "input[name=password]"
    this.password = passwordSelector
    const submitButtonSelector = "*[type=submit]"
    this.submitButton = submitButtonSelector

    this.window
        .maximize()
        .navigateTo(loginUrl)
        .waitForElementVisible("body")
        .waitForElementVisible(usernameSelector)
        .waitForElementVisible(passwordSelector)
        .assert.visible("input[name=username]")
        .sendKeys(usernameSelector, username)
        .assert.visible("input[name=password]")
        .sendKeys(passwordSelector, password)
        .assert.visible(submitButtonSelector)
        .click(submitButtonSelector)
        .pause(pause.medium)
        .agreeDemo()

    return this
}
