// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

exports.command = function () {

	this
		// .debug()
		.useXpath()
		.click("//button[@aria-label='log out button']")
		.click("//li[normalize-space()='Logout']")
		.click("//button[normalize-space()='OK']")
		.pause(2000)
		.useCss()

	return this
}
