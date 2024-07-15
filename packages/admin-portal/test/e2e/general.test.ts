// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"
import { electionEventLink } from ".."
const createElectionEvent = require("../commands/createElectionEvent");


interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
    electionEventLink: string
    electionLink: string
    contestLink: string
    candidateLink: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("login", function (this: ExtendDescribeThis<LoginThis>) {

	before(function (this: ExtendDescribeThis<LoginThis>, browser) {
		browser.login()
	})

	after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
		// Logout
		browser
			.logout()
	})

    it("create an election event", async (browser: NightwatchAPI) => {
		browser.assert.urlContains(electionEventLink)
        browser.assert
            .visible(`a.${electionEventLink!}`)
            .click(`a.${electionEventLink!}`)
            .assert.visible("input[name=name]")
            .sendKeys("input[name=name]", "this is a test election event name")
            .assert.visible("input[name=description]")
            .sendKeys("input[name=description]", "this is a test election event description")
            .assert.enabled(`button.election-event-save-button`)
            .click("button.election-event-save-button")
            .pause(5000)
            .assert.visible(`a[title='this is a test election event name']`)
    })

    it("delete an election event", async (browser: NightwatchAPI) => {
		browser.assert.urlContains(electionEventLink)
		browser.hoverAndClick({
			hoverElement: `a[title='${createElectionEvent.config.electionEventName}']`,
			clickElement: `a[title='${createElectionEvent.config.electionEventName}'] + div.menu-actions-${electionEventLink!}`
		})
        browser.assert
            .visible(`li.menu-action-delete-${electionEventLink!}`)
            .click(`li.menu-action-delete-${electionEventLink!}`)
            .assert.enabled(`button.ok-button`)
            .click("button.ok-button")
            .pause(1000)
            .assert.not.elementPresent(`a[title='this is a test election name']`)
    })
})
