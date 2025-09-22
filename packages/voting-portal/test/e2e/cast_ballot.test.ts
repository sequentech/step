// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {loginUrl, password, pause, username} from ".."
import {selectCandidatesForContest} from "../commands/selectCandidatesForContest"

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

    it("should be able to cast ballot for all available elections", async (browser: NightwatchAPI) => {
        const electionListLabel = await browser.element.findByText("Ballot List")
        browser.assert.visible(electionListLabel).pause(pause.medium)

        //get list of elections by class selector
        const electionList = await browser.elements("css selector", `.election-title`)

        // loop through found elections
        const namesOfElections = electionList.map(async function (electionItem) {
            //get name of each election and push into namesOfElections array
            const electionTitle = await browser.elementIdText(
                Object.values(electionItem)[0] as string
            )

            // namesOfElections.push(electionTitle);
            return electionTitle
        })

        const asyncFns = namesOfElections.map((nameOfElection) => {
            return () =>
                new Promise((res, rej) => {
                    nameOfElection.then((e) => {
                        browser
                            .useXpath()
                            .click(
                                `//p[contains(normalize-space(),'${e}') and contains(@class,'election-title')]/../../div/button[normalize-space()='Click to Vote']`
                            )
                            .click(`//button[normalize-space()="Start Voting"]`)

                        browser.elements("css selector", `.contest-title`, function (contestList) {
                            contestList.value.forEach((i) => selectCandidatesForContest(browser, i))

                            browser
                                .click(`//button[contains(@class,"next-button")]`)
                                .click(`//button[contains(normalize-space(),"Cast your ballot")]`)
                                .click(`//button[contains(normalize-space(),"Finish")]`)
                                .agreeDemo()

                            res("complete")
                        })
                    })
                })
        })

        for (const fn of asyncFns) {
            await fn()
        }
    })
})
