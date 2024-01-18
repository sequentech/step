import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("login", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.navigateTo(this.testUrl!)
        // perform login
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .sendKeys(this.username!, "felix")
            .sendKeys(this.password!, "felix")
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

    it("create an election event", async (browser: NightwatchAPI) => {
        browser.assert.urlContains("sequent_backend_election_event")
        browser.assert
            .visible("button.election-event-create-button")
            .click("button.election-event-create-button")
    })
})
