// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {loginUrl, password, pause, username} from ".."
import {verifyBallot} from "../../../ballot-verifier/test/commands/verifyBallot"
import {selectCandidatesForContest} from "../commands/selectCandidatesForContest"
import {getElectionId} from "../commands/getElectionId"

describe("Cast ballot", function () {
    before(function (browser) {
        browser.pause(pause.medium).login({
            loginUrl,
            username,
            password,
        })
    })

    after(function (browser) {
        browser.logout().end()
    })

    it("should be able to verify ballot for one election", async (browser: NightwatchAPI) => {
        browser
            .waitForElementVisible("body")
            .useXpath()
            .click(
                `//p[contains(@class,'election-title')]/../../div/button[normalize-space()='Click to Vote']`
            )
            .click(`//button[normalize-space()="Start Voting"]`)

        browser.elements("css selector", `.contest-title`, function (contestList) {
            contestList.value.forEach((i) => selectCandidatesForContest(browser, i))
        })

        browser.click(`//button[contains(@class,"next-button")]`)

        browser.useXpath().click(`//div[contains(text(),'Edit ballot')]/../../../button[1]`)

        browser
            .useXpath()
            .click(`//button[normalize-space()="Yes, I want to DISCARD my ballot to audit it"]`)
            .pause(pause.medium)

        const result = await browser
            .useXpath()
            .getText(
                `//div[contains(normalize-space(),'Your Ballot ID: ') and contains(@class,'hash-text')]`
            )
        console.log({result})

        const ballotId = result.split(":")[1].trim()
        console.log("Text after colon:", ballotId)

        console.log({ballotId})

        const electionId = await getElectionId(browser)
        console.log({electionId})

        const ballotPath = `${require("path").resolve(
            __dirname + "/downloads/"
        )}/${electionId}-ballot.txt`

        browser.deleteFileIfExists(ballotPath)
        browser.click(`//button[contains(.,'Download')]`)

        browser
            .execute(function () {
                document.querySelector("a.link").removeAttribute("target")
            })
            .useCss()
            .click("a.link")
            .pause(pause.medium)

        verifyBallot(browser, {
            ballotPath,
            ballotId,
        })
    })
})
