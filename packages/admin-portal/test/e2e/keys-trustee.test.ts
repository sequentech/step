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
describe("keys trustee 1 tests", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.electionEventLink = "sequent_backend_election_event"
    this.electionLink = "sequent_backend_election"
    this.contestLink = "sequent_backend_contest"
    this.candidateLink = "sequent_backend_candidate"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.window.maximize()
        browser.navigateTo(this.testUrl!)
        // perform login
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .sendKeys(this.username!, "trustee1")
            .sendKeys(this.password!, "trustee1")
            .click(this.submitButton!)
            .pause(1000)
    })

    after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
        // Logout
        browser
            .click("button.profile-menu-button")
            .click("li.logout-button")
            .click("button.ok-button")
            .end()
    })

    it("has list of keys back button", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
        const resultElement = await browser.element.findAll(
            `a.menu-item-${this.electionEventLink!}`
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
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible(".keys-start-back-button")
                        .click(".keys-start-back-button")
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                }
            }
        )
    })

    it("has list of keys next button", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
        const resultElement = await browser.element.findAll(
            `a.menu-item-${this.electionEventLink!}`
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
                    browser.assert
                        .visible(".keys-view-trustee-icon")
                        .click(".keys-view-trustee-icon")
                    browser.assert
                        .visible(".keys-start-next-button")
                        .click(".keys-start-next-button")
                }
            }
        )
    })
})
