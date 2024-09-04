// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

exports.command = function () {
    this.useXpath().click("//button[text()='I accept my vote will Not be cast']").useCss()

    return this
}
