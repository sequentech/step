// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {pause, voterDetails} from ".."
import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"

describe("voters tests", function () {
    before(function (browser) {
        browser.login()

        // create election event
        createElectionEvent.createElectionEvent(browser)
        createElectionEvent.createElection(browser)
        createElectionEvent.createContest(browser)
        createElectionEvent.createCandidates(browser)
    })

    after(async function (browser) {
        //delete election event
        deleteElectionEvent.deleteCandidates(browser)
        deleteElectionEvent.deleteContest(browser)
        deleteElectionEvent.deleteElection(browser)
        deleteElectionEvent.deleteElectionEvent(browser)

        // Logout
        browser.logout()
    })

    it("create a voter", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.assert
                        .visible("button.voter-add-button")
                        .click("button.voter-add-button")
                } else {
                    browser.assert.visible("button.add-button").click("button.add-button")
                }

                browser
                    .sendKeys("input[name=first_name]", voterDetails.firstName)
                    .sendKeys("input[name=last_name]", voterDetails.lastName)
                    .sendKeys("input[name=email]", voterDetails.email)
                    .sendKeys("input[name=username]", voterDetails.username)
                    .assert.enabled("button[type=submit]")
                    .click("button[type=submit]")
                    .pause(pause.short)
                // .debug()

                browser.useXpath()

                //assert voter created with proper details
                browser.assert.visible(
                    `//span[contains(@class, 'first_name') and text()='${voterDetails.firstName}']`
                )
                browser.assert.visible(
                    `//span[contains(@class, 'last_name') and text()='${voterDetails.lastName}']`
                )
                browser.assert.visible(
                    `//span[contains(@class, 'email') and text()='${voterDetails.email}']`
                )
                //voter details is for some reason
                browser.assert.visible(
                    `//span[contains(@class, 'username') and text()='${voterDetails.email}']`
                )

                browser.useCss()
            }
        )
    })

    it("edit a voter to set password", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
                    browser
                        .sendKeys("input[name=password]", "secretepassword")
                        .sendKeys("input[name=repeat_password]", "secretepassword")
                        .assert.enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(pause.short)

                    browser
                        .useXpath()
                        .assert.visible(
                            `//span[contains(@class, 'first_name') and text()='${voterDetails.firstName}']`
                        )
                        .useCss()
                }
            }
        )
    })

    it("edit a voter to set area", async (browser: NightwatchAPI) => {
        // create area
        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.assert.visible("button.area-add-button").click("button.area-add-button")
                } else {
                    browser.assert.visible("button.add-button").click("button.add-button")
                }
                browser
                    .sendKeys("input[name=name]", "this is an area name")
                    .assert.enabled("button[type=submit]")
                    .click("button[type=submit]")
                    .pause(pause.short)

                browser
                    .useXpath()
                    .assert.visible(
                        `//span[contains(@class, 'area-name') and text()='this is an area name']`
                    )
                    .useCss()
            }
        )

        // activate voters tab
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
                    browser.assert.visible(".select-voter-area").click(".select-voter-area")
                    const opcion = await browser.element.findByRole("option")
                    opcion.click()
                    browser.assert
                        .enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(pause.short)

                    browser
                        .useXpath()
                        .assert.visible(
                            `//span[contains(@class, 'first_name') and text()='${voterDetails.firstName}']`
                        )
                        .useCss()
                }
            }
        )

        // delete area
        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".delete-area-icon").click(".delete-area-icon")
                    browser.assert
                        .enabled(`button.ok-button`)
                        .click("button.ok-button")
                        .pause(pause.short)
                        .assert.not.elementPresent("span.area-description")
                }
            }
        )
    })

    it("delete a voter", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.useXpath()
                    browser.assert
                        .visible(
                            `//span[normalize-space()='${voterDetails.firstName}']/../../td/button[contains(@class,'delete-voter-icon')]`
                        )
                        .click(
                            `//span[normalize-space()='${voterDetails.firstName}']/../../td/button[contains(@class,'delete-voter-icon')]`
                        )
                    browser.useCss()
                    browser.assert
                        .enabled(`button.ok-button`)
                        .click("button.ok-button")
                        .pause(pause.short)
                        .useXpath()
                        .assert.not.elementPresent(
                            `//span[contains(@class, 'first_name') and text()='${voterDetails.firstName}']`
                        )
                        .useCss()
                }
            }
        )
    })
})
