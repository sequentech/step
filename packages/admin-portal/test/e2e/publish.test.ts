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
describe("publish tests", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.electionEventLink = "sequent_backend_election_event"
    this.electionLink = "sequent_backend_election"
    this.contestLink = "sequent_backend_contest"
    this.candidateLink = "sequent_backend_candidate"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.navigateTo(this.testUrl!)
        // perform login
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .sendKeys(this.username!, "admin")
            .sendKeys(this.password!, "admin")
            .click(this.submitButton!)
            .pause(1000)

        browser.element(`a.${this.electionEventLink!}`).click()
        browser.element("input[name=name]").sendKeys("this is a test election event name")
        browser.element("button.election-event-save-button").click()
        browser.pause(5000)

        // create an election"
        browser.element(`a.${this.electionLink!}`).click()
        browser.element("input[name=name]").sendKeys("this is a test election name")
        browser.element("button.election-save-button").click()

        // create a contest"
        browser.element(`a.${this.contestLink!}`).click()
        browser.element("input[name=name]").sendKeys("this is a test contest name")
        browser.element("button.contest-save-button").click()

        // create a candidate one"
        browser.element(`a.${this.candidateLink!}`).click()
        browser.element("input[name=name]").sendKeys("this is candidate one name")
        browser.element("button.candidate-save-button").click()
        browser.pause(500)

        // create a candidate two"
        browser.element(`a.${this.candidateLink!}`).click()
        browser.element("input[name=name]").sendKeys("this is candidate two name")
        browser.element("button.candidate-save-button").click()
        browser.pause(500)
    })

    after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
        // delete candidate one
        let menu = await browser
            .element(
                `a[title='this is candidate one name'] + div.menu-actions-${this.candidateLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.element(`li.menu-action-delete-${this.candidateLink!}`).click()
        browser.pause(200)
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // delete candidate two
        menu = await browser
            .element(
                `a[title='this is candidate two name'] + div.menu-actions-${this.candidateLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.pause(200)
        browser.element(`li.menu-action-delete-${this.candidateLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // delete contest
        menu = await browser
            .element(
                `a[title='this is a test contest name'] + div.menu-actions-${this.contestLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.pause(200)
        browser.element(`li.menu-action-delete-${this.contestLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // delete election
        menu = await browser
            .element(
                `a[title='this is a test election name'] + div.menu-actions-${this.electionLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.pause(200)
        browser.element(`li.menu-action-delete-${this.electionLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // delete election event
        menu = await browser
            .element(
                `a[title='this is a test election event name'] + div.menu-actions-${this
                    .electionEventLink!}`
            )
            .moveTo()
        browser.click(menu)
        browser.pause(200)
        browser.element(`li.menu-action-delete-${this.electionEventLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // Logout
        browser
            .click("button.profile-menu-button")
            .click("li.logout-button")
            .click("button.ok-button")
            .end()
    })

    it("create a publish", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a.menu-item-${this.electionEventLink!}`
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
                browser.pause(5000)
                browser.assert
                    .enabled("button.publish-publish-button")
                    .click("button.publish-publish-button")
                    .pause(200)
                    .assert.not.enabled("button.publish-action-publish-button")
            }
        )
    })

    it("publish view can go back", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a.menu-item-${this.electionEventLink!}`
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
                        .visible("publish-visibility-icon")
                        .click("publish-visibility-icon")
                }
                browser.assert
                    .enabled("button.publish-back-button")
                    .click("button.publish-back-button")
                    .pause(200)
                    .assert.not.enabled("button.publish-action-publish-button")
            }
        )
    })

    // it("edit a publish to set password", async (browser: NightwatchAPI) => {
    //     const resultElement = await browser.element.findAll(
    //         `a.menu-item-${this.electionEventLink!}`
    //     )
    //     resultElement[resultElement.length - 1].click()
    //
    //     browser.assert.visible("a.election-event-publish-tab").click("a.election-event-voter-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.publish-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".edit-publish-icon").click(".edit-voter-icon")
    //                 browser
    //                     .sendKeys("input[name=password]", "secretepassword")
    //                     .sendKeys("input[name=repeat_password]", "secretepassword")
    //                     .assert.enabled("button[type=submit]")
    //                     .click("button[type=submit]")
    //                     .pause(200)
    //                     .assert.textContains("span.first_name", "this is an publish firstname")
    //             }
    //         }
    //     )
    // })
    //
    // it("edit a publish to set area", async (browser: NightwatchAPI) => {
    //     // create area
    //     browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.area-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.assert.visible("button.area-add-button").click("button.area-add-button")
    //             } else {
    //                 browser.assert.visible("button.add-button").click("button.add-button")
    //             }
    //             browser
    //                 .sendKeys("input[name=name]", "this is an area name")
    //                 .assert.enabled("button[type=submit]")
    //                 .click("button[type=submit]")
    //                 .pause(200)
    //                 .assert.textContains("span.area-name", "this is an area name")
    //         }
    //     )
    //
    //     // activate publishs tab
    //     const resultElement = await browser.element.findAll(
    //         `a.menu-item-${this.electionEventLink!}`
    //     )
    //     resultElement[resultElement.length - 1].click()
    //
    //     browser.assert.visible("a.election-event-publish-tab").click("a.election-event-voter-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.publish-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         async (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".edit-publish-icon").click(".edit-voter-icon")
    //                 browser.assert.visible(".select-publish-area").click(".select-voter-area")
    //                 const opcion = await browser.element.findByRole("option")
    //                 opcion.click()
    //                 browser.assert
    //                     .enabled("button[type=submit]")
    //                     .click("button[type=submit]")
    //                     .pause(200)
    //                     .assert.textContains("span.first_name", "this is an publish firstname")
    //             }
    //         }
    //     )
    //
    //     // delete area
    //     browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.area-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".delete-area-icon").click(".delete-area-icon")
    //                 browser.assert
    //                     .enabled(`button.ok-button`)
    //                     .click("button.ok-button")
    //                     .pause(1000)
    //                     .assert.not.elementPresent("span.area-description")
    //             }
    //         }
    //     )
    // })
    //
    // it("delete a publish", async (browser: NightwatchAPI) => {
    //     const resultElement = await browser.element.findAll(
    //         `a.menu-item-${this.electionEventLink!}`
    //     )
    //     resultElement[resultElement.length - 1].click()
    //
    //     browser.assert.visible("a.election-event-publish-tab").click("a.election-event-voter-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.publish-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".delete-publish-icon").click(".delete-voter-icon")
    //                 browser.assert
    //                     .enabled(`button.ok-button`)
    //                     .click("button.ok-button")
    //                     .pause(1000)
    //                     .assert.not.elementPresent("span.first_name")
    //             }
    //         }
    //     )
    // })
})
