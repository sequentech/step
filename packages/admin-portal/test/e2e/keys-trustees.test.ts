// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {createElectionEvent} from "../commands/election-event/create"
import {pause} from ".."

// eslint-disable-next-line jest/valid-describe-callback
describe("keys trustee 1 tests", function () {
    before(function (browser) {
        // login
        browser.login()
    })

    after(async function (browser) {
        // Logout
        browser.logout()
    })

    it("has list of keys back button", async (browser: NightwatchAPI) => {
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-back-button")
                        .click("button.keys-start-back-button")
                    browser.assert.visible(".keys-view-trustee-icon")
                }
            }
        )
    })

    it("has list of keys start next button", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-next-button")
                        .click("button.keys-start-next-button")
                }
            }
        )
    })

    it("has list of keys download back button", async (browser: NightwatchAPI) => {
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-next-button")
                        .click("button.keys-start-next-button")
                    browser.assert
                        .visible("button.keys-download-back-button")
                        .click("button.keys-download-back-button")

                    browser.assert.visible("a.election-keys-tab").click("a.election-keys-tab")
                    browser.assert.visible(".keys-view-trustee-icon")
                }
            }
        )
    })

    it("has list of keys download button", async (browser: NightwatchAPI) => {
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-next-button")
                        .click("button.keys-start-next-button")
                    browser.assert
                        .visible("button.keys-download-download-button")
                        .click("button.keys-download-download-button")
                    browser.assert.visible(".keys-download-success")
                    browser
                        .isEnabled("button.keys-download-next-button")
                        .click("button.keys-download-next-button")
                    browser.assert
                        .visible(".keys-download-first-checkbox")
                        .click(".keys-download-first-checkbox > input[type=checkbox]")
                    browser.assert
                        .visible(".keys-download-second-checkbox")
                        .click(".keys-download-second-checkbox > input[type=checkbox]")
                    browser.assert
                        .visible("button.ok-button")
                        .click("button.ok-button")

                        .pause(pause.medium)
                }
            }
        )
    })

    it("has list of keys check button", async (browser: NightwatchAPI) => {
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
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-next-button")
                        .click("button.keys-start-next-button")
                    browser.assert
                        .visible("button.keys-download-download-button")
                        .click("button.keys-download-download-button")
                    browser.assert.visible(".keys-download-success")
                    browser
                        .isEnabled("button.keys-download-next-button")
                        .click("button.keys-download-next-button")
                    browser.assert
                        .visible(".keys-download-first-checkbox")
                        .click(".keys-download-first-checkbox > input[type=checkbox]")
                    browser.assert
                        .visible(".keys-download-second-checkbox")
                        .click(".keys-download-second-checkbox > input[type=checkbox]")
                    browser.assert.visible("button.ok-button").click("button.ok-button")
                    browser.assert.visible(".drop-file-dropzone")
                    browser.assert
                        .visible("button.keys-check-next-button")

                        .pause(pause.medium)
                }
            }
        )
    })

    it("has list of keys finish screen", async (browser: NightwatchAPI) => {
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible("button.keys-start-next-button")
                        .click("button.keys-start-next-button")
                    browser.assert
                        .visible("button.keys-download-download-button")
                        .click("button.keys-download-download-button")
                    browser.assert.visible(".keys-download-success")
                    browser
                        .isEnabled("button.keys-download-next-button")
                        .click("button.keys-download-next-button")
                    browser.assert
                        .visible(".keys-download-first-checkbox")
                        .click(".keys-download-first-checkbox > input[type=checkbox]")
                    browser.assert
                        .visible(".keys-download-second-checkbox")
                        .click(".keys-download-second-checkbox > input[type=checkbox]")
                    browser.assert.visible("button.ok-button").click("button.ok-button")
                    browser.assert.visible(".drop-file-dropzone")
                    browser.assert
                        .visible("button.keys-check-next-button")
                        // .click("button.keys-check-next-button")

                        .pause(pause.medium)
                }
            }
        )
    })
    it("has list of keys check button 2", async (browser: NightwatchAPI) => {
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
                if (!result.value) {
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
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
