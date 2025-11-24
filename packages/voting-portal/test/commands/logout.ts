// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {pause} from ".."

exports.command = function () {
    this
        // .debug()
        .useXpath()
        .click("//button[@aria-label='log out button']")
        .click("//li[normalize-space()='Logout']")
        .click("//button[normalize-space()='OK']")
        .pause(pause.medium)
        .useCss()

    return this
}
