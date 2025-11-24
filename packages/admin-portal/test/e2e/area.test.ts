// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"
import {pause} from ".."

describe("areas tests", function () {
    before(function (browser) {
        // login
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

    it("create an area", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
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
                    .assert.textContains("span.area-name", "this is an area name")
            }
        )
    })

    it("edit an area", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

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
                    browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
                    browser
                        .sendKeys("input[name=description]", "this is an area description")
                        .assert.enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(pause.short)
                        .assert.textContains("span.area-description", "this is an area description")
                }
            }
        )
    })

    it("edit an area contest", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
                    browser.click("#area_contest_ids")

                    const option = await browser.element.findByRole("option")
                    option.click()

                    browser.assert
                        .enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(pause.short)
                }
            }
        )
    })

    it("edit an area contest unset contest", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

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
                    browser.assert.visible(".edit-area-icon").click(".edit-area-icon")
                    browser
                        .useXpath()
                        .click(
                            "//div[span[text()='this is a test contest name']]//*[local-name()='svg']"
                        )
                        .useCss()
                        .assert.enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(pause.short)
                }
            }
        )
    })

    it("delete an area", async (browser: NightwatchAPI) => {
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
})
