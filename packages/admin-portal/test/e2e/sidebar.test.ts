// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {contestLink, electionEventLink, electionLink, pause} from ".."

describe("sidebar tests", function () {
    before(function (browser) {
        browser.login()
    })

    after(async function (browser) {
        // Logout
        browser.logout()
    })

    it("click on an election event", async (browser: NightwatchAPI) => {
        // checks if has election menu
        // case yes opens election menu
        // case no asserts create election is visible
        browser.isPresent(
            {
                selector: `a.menu-item-${electionLink}`,
                supressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    // opens election menu
                    browser.assert
                        .visible(`a.menu-item-${electionLink}`)
                        .click(
                            `div.menu-item-toggle-${electionLink}:has(+ a.menu-item-${electionLink})`
                        )

                    // checks if has contest menu
                    // case yes opens contest menu
                    // case no asserts create contest is visible
                    browser.isPresent(
                        {
                            selector: `a.menu-item-${contestLink}`,
                            supressNotFoundErrors: true,
                            timeout: 1000,
                        },
                        (result) => {
                            if (result.value) {
                                // opens contest menu
                                browser.assert
                                    .visible(`a.menu-item-${contestLink}`)
                                    .click(
                                        `div.menu-item-toggle-${contestLink}:has(+ a.menu-item-${electionLink})`
                                    )
                            } else {
                                // check has new
                                browser.assert.visible(`a.${contestLink!}`)
                            }
                        }
                    )

                    // closes election menu
                    browser.assert
                        .visible(`a.menu-item-${electionLink}`)
                        .click(
                            `div.menu-item-toggle-${electionLink}:has(+ a.menu-item-${electionLink})`
                        )

                    // closes ee menu
                    browser.assert
                        .visible("a.menu-item-active")
                        .click(
                            `div.menu-item-toggle-${electionEventLink}:has(+ a.menu-item-active)`
                        )
                } else {
                    browser.assert.visible(`a.${electionLink!}`)
                    // .click(`a.${electionLink!}`)
                    browser.assert
                        .visible("a.menu-item-active")
                        .click(
                            `div.menu-item-toggle-${electionEventLink}:has(+ a.menu-item-active)`
                        )
                }
            }
        )

        browser.pause(pause.medium)
        // .visible(`a.${electionLink!}`)
        // .click(`a.${electionLink!}`)

        // await browser.debug()
    })

    // it("create an election", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_election_event")
    //     browser.assert
    //         .visible(`a.${electionLink!}`)
    //         .click(`a.${electionLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is a test election name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is a test election description")
    //         .assert.enabled(`button.election-save-button`)
    //         .click("button.election-save-button")
    //         .pause(pause.short)
    //         .assert.visible(`a[title='this is a test election name']`)
    // })
    //
    // it("create a contest", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_election")
    //     browser.assert
    //         .visible(`a.${contestLink!}`)
    //         .click(`a.${contestLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is a test contest name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is a test contest description")
    //         .assert.enabled(`button.contest-save-button`)
    //         .click("button.contest-save-button")
    //         .pause(pause.short)
    //         .assert.visible(`a[title='this is a test contest name']`)
    // })
    //
    // it("create a candidate one", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_contest")
    //     browser.assert
    //         .visible(`a.${candidateLink!}`)
    //         .click(`a.${candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate one name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate one description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(pause.short)
    //         .assert.visible(`a[title='this is candidate one name']`)
    // })
    //
    // it("create a candidate two", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_candidate")
    //     browser.assert
    //         .visible(`a.${candidateLink!}`)
    //         .click(`a.${candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate two name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate two description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(pause.short)
    //         .assert.visible(`a[title='this is candidate two name']`)
    // })
    //
    // it("create a new area", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_candidate")
    //     browser.assert
    //         .visible(`a.${candidateLink!}`)
    //         .click(`a.${candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate two name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate two description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(pause.short)
    //         .assert.visible(`a[title='this is candidate two name']`)
    // })
    //
    // it("delete candidate one", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is candidate one name'] + div.menu-actions-${candidateLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${candidateLink!}`)
    //         .click(`li.menu-action-delete-${candidateLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(pause.short)
    //         .assert.not.elementPresent(`a[title='this is candidate one name']`)
    // })
    // it("delete candidate two", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is candidate two name'] + div.menu-actions-${candidateLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${candidateLink!}`)
    //         .click(`li.menu-action-delete-${candidateLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(pause.short)
    //         .assert.not.elementPresent(`a[title='this is candidate two name']`)
    // })
    // it("delete contest", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is a test contest name'] + div.menu-actions-${contestLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${contestLink!}`)
    //         .click(`li.menu-action-delete-${contestLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(pause.short)
    //         .assert.not.elementPresent(`a[title='this is a test contest name`)
    // })
    // it("delete an election", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is a test election name'] + div.menu-actions-${electionLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${electionLink!}`)
    //         .click(`li.menu-action-delete-${electionLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(pause.short)
    //         .assert.not.elementPresent(`a[title='this is a test election name']`)
    // })
    // it("delete an election event", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is a test election event name'] + div.menu-actions-${this
    //                 .electionEventLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${electionEventLink!}`)
    //         .click(`li.menu-action-delete-${electionEventLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(pause.short)
    //         .assert.not.elementPresent(`a[title='this is a test election event name']`)
    // })
})
