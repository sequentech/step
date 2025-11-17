// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"
import {pause} from ".."

describe("keys tests", function () {
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

    it("create a publish", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-publish-tab").click("a.election-event-publish-tab")

        browser.isPresent(
            {
                selector: "button.publish-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.assert
                        .visible("button.publish-add-button")
                        .click("button.publish-add-button")
                }
                browser.pause(pause.long)
                browser.assert
                    .enabled("button.publish-publish-button")
                    .click("button.publish-publish-button")
                    .pause(pause.short)
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.not.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("start new key generation", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")

        browser.isPresent(
            {
                selector: "button.keys-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.assert.visible("button.keys-add-button").click("button.keys-add-button")
                }
                browser.assert.enabled("button.keys-back-button").click("button.keys-back-button")
            }
        )
    })

    it("start new key generation should throw error if one trustee", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")

        browser.isPresent(
            {
                selector: "button.keys-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.assert.visible("button.keys-add-button").click("button.keys-add-button")
                    browser.click("#trusteeNames_trustee1")
                    browser.assert
                        .enabled("button.keys-create-button")
                        .click("button.keys-create-button")
                    browser.assert.textEquals(
                        ".keys-trustees-input > p",
                        "You selected only 1 trustee, but you must select at least 2."
                    )
                }
            }
        )
    })

    it("start new key generation should continue if two trustees", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")

        browser.isPresent(
            {
                selector: "button.keys-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.assert.visible("button.keys-add-button").click("button.keys-add-button")
                    browser.click("#trusteeNames_trustee1")
                    browser.click("#trusteeNames_trustee2")
                    browser.assert
                        .enabled("button.keys-create-button")
                        .click("button.keys-create-button")
                    browser.assert
                        .enabled(`button.ok-button`)
                        .click("button.ok-button")
                        .pause(pause.short)
                        .assert.not.elementPresent("span.area-description")
                    browser.assert.visible(".keys-ceremony-title")
                }
            }
        )
    })

    it("has list of keys", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")

        browser.isPresent(
            {
                selector: "button.keys-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".keys-view-admin-icon").click(".keys-view-admin-icon")
                    browser.waitUntil(async () => {
                        const status = await browser
                            .element(".keys-ceremony-status > span")
                            .getText()
                        return status.includes("NOT_STARTED")
                    })
                    browser.assert.textContains(".keys-ceremony-status > span", "NOT_STARTED")
                }
            }
        )
    })

    it("keys status is in process", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEvent.name}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")

        browser.isPresent(
            {
                selector: "button.keys-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".keys-view-admin-icon").click(".keys-view-admin-icon")
                    browser.waitUntil(async () => {
                        const status = await browser
                            .element(".keys-ceremony-status > span")
                            .getText()
                        return status.includes("IN_PROCESS")
                    })
                    browser.assert.textContains(".keys-ceremony-status > span", "IN_PROCESS")
                }
            }
        )
    })
})
