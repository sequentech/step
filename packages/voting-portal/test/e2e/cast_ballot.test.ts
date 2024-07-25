// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {loginUrl, password, pause, username} from ".."

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

        // let namesOfElections = []

        //get list of elections by class selector
        const electionList = await browser.elements("css selector", `.election-title`)
        console.log({electionList})

        // loop through found elections
        const namesOfElections = electionList.map(async function (electionItem) {
            console.log({electionItem})

            //get name of each election and push into namesOfElections array
            const electionTitle = await browser.elementIdText(
                Object.values(electionItem)[0] as string
            )

            console.log({eTitle: electionTitle})
            // namesOfElections.push(electionTitle);
            return electionTitle
        })

        console.log({namesOfElections})

        const asyncFns = namesOfElections.map((nameOfElection) => {
            console.log({nameOfElection})
            return () =>
                new Promise((res, rej) => {
                    nameOfElection.then((e) => {
                        console.log({e})
                        browser
                            .useXpath()
                            .click(
                                `//p[contains(normalize-space(),'${e}') and contains(@class,'election-title')]/../../div/button[normalize-space()='Click to Vote']`
                            )
                            .click(`//button[normalize-space()="Start Voting"]`)

                        browser.elements("css selector", `.contest-title`, function (contestList) {
                            console.log({contestList})

                            contestList.value.forEach((contestItem) => {
                                console.log({contestItem})

                                browser.elementIdText(
                                    Object.values(contestItem)[0] as string,
                                    function (contestTitle) {
                                        console.log({contestTitle})
                                        browser.elements(
                                            "xpath",
                                            `//h5[normalize-space()='${contestTitle.value}']/..//div[contains(@class, 'candidate-item')]`,
                                            function (candidateList) {
                                                // console.log({res})
                                                candidateList.value.forEach(function (el) {
                                                    console.log({el})
                                                })

                                                browser
                                                    .useXpath()
                                                    .click(
                                                        `//h5[normalize-space()='${
                                                            contestTitle.value
                                                        }']/..//div[contains(@class, 'candidate-item')][${
                                                            Math.floor(
                                                                Math.random() *
                                                                    candidateList.value.length
                                                            ) + 1
                                                        }]`
                                                    )
                                            }
                                        )
                                    }
                                )
                            })

                            browser
                                .click(`//button[contains(@class,"next-button")]`)
                                .click(`//button[contains(normalize-space(),"Cast your ballot")]`)
                                .click(`//button[contains(normalize-space(),"Finish")]`)
                                .agreeDemo()

                            console.log("finished ", {e})
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
