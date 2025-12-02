// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {pause} from ".."

exports.command = function () {
    this.click("header [data-testid='AccountCircleIcon']")
        .click("li.logout-button")
        .click("button.ok-button")
        .pause(pause.medium)

    return this
}
