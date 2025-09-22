// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {admin_portal_password, admin_portal_username, NightWatchLogin, pause, testUrl} from ".."

exports.command = function (
    username = admin_portal_username,
    password = admin_portal_password
): NightWatchLogin {
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.window
        .maximize()
        .navigateTo(testUrl)
        .waitForElementVisible(this.username)
        .waitForElementVisible(this.password)
        .assert.visible("input[name=username]")
        .sendKeys(this.username, username)
        .assert.visible("input[name=password]")
        .sendKeys(this.password, password)
        .pause(pause.medium)
        .assert.visible(this.submitButton)
        .click(this.submitButton)
        .pause(pause.medium)

    return this
}
