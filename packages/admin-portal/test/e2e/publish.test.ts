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
        browser.element(`li.menu-action-archive-${this.electionEventLink!}`).click()
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
        await browser.window.maximize()
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
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.not.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("publish view can go back", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                        .visible(".publish-visibility-icon")
                        .click(".publish-visibility-icon")
                }
                browser.assert
                    .enabled("button.publish-back-button")
                    .click("button.publish-back-button")
                    .pause(200)
                    .assert.not.enabled("button.publish-action-pause-button")
                    .assert.not.enabled("button.publish-action-stop-button")
            }
        )
    })

    it("publish can start election", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
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
                    browser.assert.visible(".publish-visibility-icon")
                    browser.assert
                        .enabled("button.publish-action-start-button")
                        .click("button.publish-action-start-button")
                        .pause(200)
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
