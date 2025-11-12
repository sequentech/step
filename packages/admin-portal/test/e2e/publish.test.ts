// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"

import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"
import {pause} from ".."

describe("publish tests", function () {
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
        await browser.window.maximize()
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

    it("publish view can go back", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                    browser.end()
                } else {
                    browser.assert
                        .visible(".publish-visibility-icon")
                        .click(".publish-visibility-icon")
                }
                browser.assert
                    .enabled("button.publish-back-button")
                    .click("button.publish-back-button")
                    .pause(pause.short)
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.not.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("publish can start election", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                    browser.end()
                } else {
                    browser.assert.visible(".publish-visibility-icon")
                    browser.assert
                        .enabled("button.publish-action-start-button")
                        .click("button.publish-action-start-button")
                        .pause(pause.short)
                    browser.assert.enabled("button.ok-button").click("button.ok-button")
                }
                browser.assert.not
                    .enabled("button.publish-action-start-button")
                    .assert.enabled("button.publish-action-pause-button")
                    .assert.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("publish can pause election", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                    browser.end()
                } else {
                    browser.assert.visible(".publish-visibility-icon")
                    browser.assert
                        .enabled("button.publish-action-start-button")
                        .click("button.publish-action-start-button")
                    browser.assert.enabled("button.ok-button").click("button.ok-button")
                    browser.assert
                        .enabled("button.publish-action-pause-button")
                        .click("button.publish-action-pause-button")
                    browser.assert.enabled("button.ok-button").click("button.ok-button")
                }
                browser.assert
                    .enabled("button.publish-action-start-button")
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("publish can stop election", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                    browser.end()
                } else {
                    browser.assert.visible(".publish-visibility-icon")
                    browser.assert
                        .enabled("button.publish-action-start-button")
                        .click("button.publish-action-start-button")
                    browser.assert.enabled("button.ok-button").click("button.ok-button")
                    browser.assert
                        .enabled("button.publish-action-stop-button")
                        .click("button.publish-action-stop-button")
                    browser.assert.enabled("button.ok-button").click("button.ok-button")
                }
                browser.assert.not
                    .enabled("button.publish-action-start-button")
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.not.enabled("button.publish-action-stop-button")
            }
        )
    })
})
