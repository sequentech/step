// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"

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
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.electionEventLink = "sequent_backend_election_event"
    this.electionLink = "sequent_backend_election"
    this.contestLink = "sequent_backend_contest"
    this.candidateLink = "sequent_backend_candidate"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.navigateTo(this.testUrl!)
        // perform login
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .sendKeys(this.username!, "admin")
            .sendKeys(this.password!, "admin")
            .click(this.submitButton!)
            .pause(1000)
    })

    after(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser
            .click("header [data-testid='AccountCircleIcon']")
            .click("li.logout-button")
            .click("button.ok-button")
            .end()
    })

    it("create an election event", async (browser: NightwatchAPI) => {
        browser.assert.urlContains("sequent_backend_election_event")
        browser.assert
            .visible(`a.${this.electionEventLink!}`)
            .click(`a.${this.electionEventLink!}`)
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
        browser.assert.urlContains("sequent_backend_election_event")
        // browser.debug()
        const menu = await browser
            .element(
                `a[title='this is a test election event name'] + div.menu-actions-${this
                    .electionEventLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.assert
            .visible(`li.menu-action-delete-${this.electionEventLink!}`)
            .click(`li.menu-action-delete-${this.electionEventLink!}`)
            .assert.enabled(`button.ok-button`)
            .click("button.ok-button")
            .pause(1000)
            .assert.not.elementPresent(`a[title='this is a test election name']`)
    })
})
