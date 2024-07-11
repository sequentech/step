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

describe("DemoRecordingWithReusedLogic2", function (this: ExtendDescribeThis<LoginThis>) {
    this.electionEventLink = "sequent_backend_election_event"
    this.electionLink = "sequent_backend_election"
    this.contestLink = "sequent_backend_contest"
    this.candidateLink = "sequent_backend_candidate"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
		//ts ignores to be removed
        // @ts-ignore
        browser.login("admin", "admin")
    })

    after(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser
            // @ts-ignore
            .logout()
            .end()
    })

    it("create an election event", async (browser: NightwatchAPI) => {
        browser.assert.urlContains("sequent_backend_election_event")

        browser
            .click("div.css-9t7ah8 > a")
            .assert.visible("input[name=name]")
            .assert.visible("input[name=description]")
            .click("#name")
            .setValue("#name", "Demo election event")
            .click("div.ra-input-description")
            .setValue("#description", "Demo election event description")
            .click("#main-content button")
            .assert.visible(`a[title='Demo election event']`)
    })

    it("create an election", async (browser: NightwatchAPI) => {
        browser
            .click(
                "html > body > #root > div > div:nth-of-type(1) > div > div > main > div.MuiDrawer-root > div > ul > div.css-rwj1r7 > div.css-1vay9pl > div.css-1jpm9pd > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div > div > [data-testid='AddIcon'] > path"
            )
            .click(
                "html > body > #root > div > div:nth-of-type(1) > div > div > main > div.MuiDrawer-root > div > ul > div.css-rwj1r7 > div.css-1vay9pl > div.css-1jpm9pd > div > div > div:nth-of-type(2) > div:nth-of-type(2) > div > div > div > [data-testid='AddIcon'] > path"
            )
            .click("div.MuiDrawer-root div:nth-of-type(2) > div:nth-of-type(2) a")
            .assert.visible("input[name=name]")
            .assert.visible("input[name=description]")
            .click("#name")
            .setValue("#name", "Demo election")
            .click("#description")
            .setValue("#description", "Demo Election description")
            .click("#main-content button")
            .assert.visible(`a[title='Demo election']`)
    })

    it("create a contest", async (browser: NightwatchAPI) => {
        browser
            .click("div:nth-of-type(2) > div > div > div.MuiBox-root > div:nth-of-type(2) a")
            .click("#name")
            .assert.visible("input[name=name]")
            .assert.visible("input[name=description]")
            .setValue("#name", "Demo Contest")
            .click("#description")
            .setValue("#description", "Demo contest description")
            .click("#main-content button")
            .assert.visible(`a[title='Demo Contest']`)
    })
    it("create a candidate one", async (browser: NightwatchAPI) => {
        browser
            .click(
                "div:nth-of-type(2) > div > div > div.MuiBox-root > div:nth-of-type(2) > div > div > div.MuiBox-root > div:nth-of-type(2) a"
            )
            .click("#name")
            .assert.visible("input[name=name]")
            .assert.visible("input[name=description]")
            .setValue("#name", "Demo Candidate 1")
            .click("#description")
            .setValue("#description", "one")
            .pause(500)
            .click("#main-content button")
            .pause(1500)
            .assert.visible(`a[title='Demo Candidate 1']`)
    })
    it("create a candidate two", async (browser: NightwatchAPI) => {
        browser
            .click(
                "div:nth-of-type(2) > div > div > div.MuiBox-root > div:nth-of-type(2) > div > div > div.MuiBox-root div.css-9t7ah8 > a"
            )
            .pause(1500)
            .assert.visible("input[name=name]")
            .assert.visible("input[name=description]")
            .click("#name")
            .setValue("#name", "Demo Candidate 2")
            .click("#description")
            .setValue("#description", "two")
            .click("#main-content button")
            .pause(500)
            .assert.visible(`a[title='Demo Candidate 2']`)
    })
    it("delete candidate two", async (browser: NightwatchAPI) => {
		browser
        // @ts-ignore
            .hoverAndClick(`a[title='Demo Candidate 2'] + div.menu-actions-${this.candidateLink}`)
            .click("li.menu-action-delete-sequent_backend_candidate")
            .click("button.MuiButton-solidWarning")
            .assert.not.elementPresent(`a[title='Demo Candidate 2']`)
    })

    it("delete candidate one", async (browser: NightwatchAPI) => {
		browser
		
        // @ts-ignore
            .hoverAndClick(`a[title='Demo Candidate 1'] + div.menu-actions-${this.candidateLink}`)
            .click("li.menu-action-delete-sequent_backend_candidate")
            .click("button.MuiButton-solidWarning")
            .assert.not.elementPresent(`a[title='Demo Candidate 1']`)
    })
    it("delete contest", async (browser: NightwatchAPI) => {
		browser
        // @ts-ignore
            .hoverAndClick(`a[title='Demo Contest'] + div.menu-actions-${this.contestLink}`)
            .click("li.menu-action-delete-sequent_backend_contest > div")
            .click("button.MuiButton-solidWarning")
            .assert.not.elementPresent(`a[title='Demo Contest']`)
    })
    it("delete an election", async (browser: NightwatchAPI) => {
        browser
            //@ts-ignore
            .hoverAndClick(`a[title='Demo election'] + div.menu-actions-${this.electionLink}`)
            .click("li.menu-action-delete-sequent_backend_election")
            .click("button.MuiButton-solidWarning")
            .assert.not.elementPresent(`a[title='Demo election']`)
    })
    it("delete an election event", async (browser: NightwatchAPI) => {
		browser
        //@ts-ignore
            .hoverAndClick(
                `a[title='Demo election event'] + div.menu-actions-${this.electionEventLink}`
            )
            .click("li.menu-action-delete-sequent_backend_election_event")
            .click("button.MuiButton-solidWarning")
            .assert.not.elementPresent(`a[title='Demo election event']`)
    })
})
