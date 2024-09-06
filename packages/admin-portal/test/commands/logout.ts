// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {pause} from ".."

exports.command = function () {
    this.useXpath()
        .click(`//button[@aria-label="log out button"]`)
        .useCss()
        .click("li.logout-button")
        .click("button.ok-button")
        .pause(pause.medium)

    return this
}
