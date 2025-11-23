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
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.window
        .maximize()
        .navigateTo(loginUrl)
        .waitForElementVisible("body")
        .waitForElementVisible(this.username!)
        .waitForElementVisible(this.password!)
        .assert.visible("input[name=username]")
        .sendKeys(this.username!, username)
        .assert.visible("input[name=password]")
        .sendKeys(this.password!, password)
        .assert.visible(this.submitButton!)
        .click(this.submitButton!)
        .pause(pause.medium)
        .agreeDemo()

    return this
}
