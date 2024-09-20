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

    console.log(`login: url=${testUrl} username=${username}, password=${password}`)

    this.window
        .maximize()
        .navigateTo(testUrl)
        .getCurrentUrl((currentUrl) => {
            console.log(`login: currentUrl=${currentUrl.value}`)
        })
        .waitForElementVisible(this.username)
        .waitForElementVisible(this.password)
        .assert.visible("input[name=username]")
        .getCurrentUrl((currentUrl) => {
            console.log(`login: currentUrl=${currentUrl.value}`)
        })
        .sendKeys(this.username, username)
        .assert.visible("input[name=password]")
        .sendKeys(this.password, password)
        .pause(pause.medium)
        .assert.visible(this.submitButton)
        .click(this.submitButton)
        .pause(pause.medium)
        .useXpath()
        .waitForElementVisible("//li[contains(text(),'Active')]")
        .useCss()

    return this
}
