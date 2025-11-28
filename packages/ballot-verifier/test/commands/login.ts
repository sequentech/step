// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightWatchLogin, pause} from "../index"

exports.command = function ({loginUrl = "", username = "", password = ""}): NightWatchLogin {
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.window
        .maximize()
        .navigateTo(loginUrl)
        .waitForElementVisible(this.username!)
        .waitForElementVisible(this.password!)
        .assert.visible("input[name=username]")
        .sendKeys(this.username!, username)
        .assert.visible("input[name=password]")
        .sendKeys(this.password!, password)
        .assert.visible(this.submitButton!)
        .click(this.submitButton!)
        .pause(pause.medium)

    return this
}
