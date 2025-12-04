// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

exports.command = function () {
    this.useXpath().click("//button[text()='I accept my vote will Not be cast']").useCss()

    return this
}
