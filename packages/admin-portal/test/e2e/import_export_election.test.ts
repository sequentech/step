// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {pause} from ".."
import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"
import {getElectionEventId} from "../commands/getElectionEventId"

let exportPath

describe("import/export election event  tests", function () {
    before(function (browser) {
        browser.login()

        // create election event
        createElectionEvent.createElectionEvent(browser)
        createElectionEvent.createElection(browser)
        createElectionEvent.createContest(browser)
        createElectionEvent.createCandidates(browser)
    })

    afterEach(function (browser) {
        // delete election event
        // deleteElectionEvent.deleteCandidates(browser)
        // deleteElectionEvent.deleteContest(browser)
        // deleteElectionEvent.deleteElection(browser)
        // browser.useCss()
        deleteElectionEvent.deleteElectionEvent(browser)

        browser
            .useCss()
            .assert.not.elementPresent(
                `a[title='${createElectionEvent.config.electionEvent.name}']`
            )

        // Logout
    })

    after(function (browser) {
        browser.logout()
    })

    it("export an election event", async (browser: NightwatchAPI) => {
        // // create election event
        // createElectionEvent.createElectionEvent(browser)
        // createElectionEvent.createElection(browser)
        // createElectionEvent.createContest(browser)
        // createElectionEvent.createCandidates(browser)

        // browser.debug()

        browser
            .useXpath()
            .click(`//span[normalize-space()="${createElectionEvent.config.electionEvent.name}"]`)
            .click('//a[normalize-space()="Data"]')

        const electionEventId = await getElectionEventId(browser)
        console.log({electionEventId})

        // election-event-a1320989-c6f8-49bf-b3e0-7868629c936a-export.json
        exportPath = `${require("path").resolve(
            __dirname + "/downloads/"
        )}/election-event-${electionEventId}-export.json`

        console.log({exportPath})

        browser.deleteFileIfExists(exportPath)

        browser
            .click('//button[normalize-space()="Export"]')
            .click('//button[contains(@class,"ok-button")]')
            .pause(pause.xLong)
    })

    it("imports an election event", async (browser: NightwatchAPI) => {
        // browser.debug()

        console.log({exportPath})

        browser.useXpath().click(`//a[normalize-space()="Create an Election Event"]`)
        browser.useXpath().click(`//div[contains(text(),'Import')]`)
        browser
            .useXpath()
            .uploadFile(`//input[contains(@class,"drop-input-file")]`, exportPath, function (a) {
                console.log({a})
            })
        browser.useXpath().click(`//button[normalize-space()="Import"]`)
        browser
            .useXpath()
            .click(`//button[normalize-space()="Yes, Import without Integrity Check"]`)

        // browser.useXpath().click(`//span[normalize-space()="${createElectionEvent.config.electionEvent.name}"]`).click('//a[normalize-space()="Data"]').click('//button[normalize-space()="Export"]').click('//button[contains(@class,"ok-button")]')
    })
})
