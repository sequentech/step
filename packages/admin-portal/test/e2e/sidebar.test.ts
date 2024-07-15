import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
    electionEventLink: string
    electionLink: string
    contestLink: string
    candidateLink: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("sidebar tests", function (this: ExtendDescribeThis<LoginThis>) {
    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.login()
    })

    after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
        // Logout
        browser.logout()
    })

    it("click on an election event", async (browser: NightwatchAPI) => {
        // const resultElement = await browser.element.findAll(
        //     `a.menu-item-${this.electionEventLink!}`
        // )
        // resultElement[resultElement.length - 1].click()

        // opens ee menu
        browser.assert
            .visible("a.menu-item-active")
            .click(`div.menu-item-toggle-${this.electionEventLink}:has(+ a.menu-item-active)`)

        // checks if has election menu
        // case yes opens election menu
        // case no asserts create election is visible
        browser.isPresent(
            {
                selector: `a.menu-item-${this.electionLink}`,
                supressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    // opens election menu
                    browser.assert
                        .visible(`a.menu-item-${this.electionLink}`)
                        .click(
                            `div.menu-item-toggle-${this.electionLink}:has(+ a.menu-item-${this.electionLink})`
                        )

                    // checks if has contest menu
                    // case yes opens contest menu
                    // case no asserts create contest is visible
                    browser.isPresent(
                        {
                            selector: `a.menu-item-${this.contestLink}`,
                            supressNotFoundErrors: true,
                            timeout: 1000,
                        },
                        (result) => {
                            if (result.value) {
                                // opens contest menu
                                browser.assert
                                    .visible(`a.menu-item-${this.contestLink}`)
                                    .click(
                                        `div.menu-item-toggle-${this.contestLink}:has(+ a.menu-item-${this.electionLink})`
                                    )
                            } else {
                                // check has new
                                browser.assert.visible(`a.${this.contestLink!}`)
                            }
                        }
                    )

                    // closes election menu
                    browser.assert
                        .visible(`a.menu-item-${this.electionLink}`)
                        .click(
                            `div.menu-item-toggle-${this.electionLink}:has(+ a.menu-item-${this.electionLink})`
                        )

                    // closes ee menu
                    browser.assert
                        .visible("a.menu-item-active")
                        .click(
                            `div.menu-item-toggle-${this.electionEventLink}:has(+ a.menu-item-active)`
                        )
                } else {
                    browser.assert.visible(`a.${this.electionLink!}`)
                    // .click(`a.${this.electionLink!}`)
                    browser.assert
                        .visible("a.menu-item-active")
                        .click(
                            `div.menu-item-toggle-${this.electionEventLink}:has(+ a.menu-item-active)`
                        )
                }
            }
        )

        browser.pause(2000)
        // .visible(`a.${this.electionLink!}`)
        // .click(`a.${this.electionLink!}`)

        // await browser.debug()
    })

    // it("create an election", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_election_event")
    //     browser.assert
    //         .visible(`a.${this.electionLink!}`)
    //         .click(`a.${this.electionLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is a test election name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is a test election description")
    //         .assert.enabled(`button.election-save-button`)
    //         .click("button.election-save-button")
    //         .pause(1000)
    //         .assert.visible(`a[title='this is a test election name']`)
    // })
    //
    // it("create a contest", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_election")
    //     browser.assert
    //         .visible(`a.${this.contestLink!}`)
    //         .click(`a.${this.contestLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is a test contest name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is a test contest description")
    //         .assert.enabled(`button.contest-save-button`)
    //         .click("button.contest-save-button")
    //         .pause(1000)
    //         .assert.visible(`a[title='this is a test contest name']`)
    // })
    //
    // it("create a candidate one", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_contest")
    //     browser.assert
    //         .visible(`a.${this.candidateLink!}`)
    //         .click(`a.${this.candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate one name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate one description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(1000)
    //         .assert.visible(`a[title='this is candidate one name']`)
    // })
    //
    // it("create a candidate two", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_candidate")
    //     browser.assert
    //         .visible(`a.${this.candidateLink!}`)
    //         .click(`a.${this.candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate two name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate two description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(1000)
    //         .assert.visible(`a[title='this is candidate two name']`)
    // })
    //
    // it("create a new area", async (browser: NightwatchAPI) => {
    //     browser.assert.urlContains("sequent_backend_candidate")
    //     browser.assert
    //         .visible(`a.${this.candidateLink!}`)
    //         .click(`a.${this.candidateLink!}`)
    //         .assert.visible("input[name=name]")
    //         .sendKeys("input[name=name]", "this is candidate two name")
    //         .assert.visible("input[name=description]")
    //         .sendKeys("input[name=description]", "this is candidate two description")
    //         .assert.enabled(`button.candidate-save-button`)
    //         .click("button.candidate-save-button")
    //         .pause(1000)
    //         .assert.visible(`a[title='this is candidate two name']`)
    // })
    //
    // it("delete candidate one", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is candidate one name'] + div.menu-actions-${this.candidateLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${this.candidateLink!}`)
    //         .click(`li.menu-action-delete-${this.candidateLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(1000)
    //         .assert.not.elementPresent(`a[title='this is candidate one name']`)
    // })
    // it("delete candidate two", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is candidate two name'] + div.menu-actions-${this.candidateLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${this.candidateLink!}`)
    //         .click(`li.menu-action-delete-${this.candidateLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(1000)
    //         .assert.not.elementPresent(`a[title='this is candidate two name']`)
    // })
    // it("delete contest", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is a test contest name'] + div.menu-actions-${this.contestLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${this.contestLink!}`)
    //         .click(`li.menu-action-delete-${this.contestLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(1000)
    //         .assert.not.elementPresent(`a[title='this is a test contest name`)
    // })
    // it("delete an election", async (browser: NightwatchAPI) => {
    //     // browser.debug()
    //     const menu = await browser
    //         .element(
    //             `a[title='this is a test election name'] + div.menu-actions-${this.electionLink!}`
    //         )
    //         .moveTo()
    //     browser.click(menu)
    //     browser.assert
    //         .visible(`li.menu-action-delete-${this.electionLink!}`)
    //         .click(`li.menu-action-delete-${this.electionLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(1000)
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
    //         .visible(`li.menu-action-delete-${this.electionEventLink!}`)
    //         .click(`li.menu-action-delete-${this.electionEventLink!}`)
    //         .assert.enabled(`button.ok-button`)
    //         .click("button.ok-button")
    //         .pause(1000)
    //         .assert.not.elementPresent(`a[title='this is a test election event name']`)
    // })
})
