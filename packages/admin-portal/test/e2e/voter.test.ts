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
describe("voters tests", function (this: ExtendDescribeThis<LoginThis>) {
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
    })

    after(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser
            .click("button.profile-menu-button")
            .click("li.logout-button")
            .click("button.ok-button")
            .end()
    })

    it("create an voter", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a.menu-item-${this.electionEventLink!}`
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
                    browser.assert
                        .visible("button.voter-add-button")
                        .click("button.area-add-button")
                } else {
                    browser.assert.visible("button.add-button").click("button.add-button")
                }
                browser
                    .sendKeys("input[name=first_name]", "this is an voter firstname")
                    .sendKeys("input[name=last_name]", "this is an voter lastname")
                    .sendKeys("input[name=email]", "this is an voter email")
                    .sendKeys("input[name=username]", "this is an voter username")
                    .assert.enabled("button[type=submit]")
                    .click("button[type=submit]")
                    .pause(200)
                    .assert.textContains("span.voter-first_name", "this is an voter firstname")
            }
        )
    })

    // it("edit an voter", async (browser: NightwatchAPI) => {
    //     const resultElement = await browser.element.findAll(
    //         `a.menu-item-${this.electionEventLink!}`
    //     )
    //     resultElement[resultElement.length - 1].click()
    //
    //     browser.assert.visible("a.election-event-voter-tab").click("a.election-event-area-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.voter-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".edit-voter-icon").click(".edit-area-icon")
    //                 browser
    //                     .sendKeys("input[name=description]", "this is an voter description")
    //                     .assert.enabled("button[type=submit]")
    //                     .click("button[type=submit]")
    //                     .pause(200)
    //                     .assert.textContains(
    //                         "span.voter-description",
    //                         "this is an area description"
    //                     )
    //             }
    //         }
    //     )
    // })
    //
    // it("delete an voter", async (browser: NightwatchAPI) => {
    //     const resultElement = await browser.element.findAll(
    //         `a.menu-item-${this.electionEventLink!}`
    //     )
    //     resultElement[resultElement.length - 1].click()
    //
    //     browser.assert.visible("a.election-event-voter-tab").click("a.election-event-area-tab")
    //
    //     browser.isPresent(
    //         {
    //             selector: "button.voter-add-button",
    //             suppressNotFoundErrors: true,
    //             timeout: 1000,
    //         },
    //         (result) => {
    //             if (result.value) {
    //                 browser.end()
    //             } else {
    //                 browser.assert.visible(".delete-voter-icon").click(".delete-area-icon")
    //                 browser.assert
    //                     .enabled(`button.ok-button`)
    //                     .click("button.ok-button")
    //                     .pause(1000)
    //                     .assert.not.elementPresent("span.voter-description")
    //             }
    //         }
    //     )
    // })
})
