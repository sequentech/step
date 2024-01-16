/* eslint-disable testing-library/await-async-query */
import {ExtendDescribeThis} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("login", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl = "http://localhost:3000"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    beforeEach(function (this: ExtendDescribeThis<LoginThis>, browser) {
        // navigate to the login page
        browser.navigateTo(this.testUrl!)
        // perform login
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .sendKeys(this.username!, "felix")
            .sendKeys(this.password!, "felix")
            .click(this.submitButton!)
            .pause(1000)
            .sendKeys("#code", "123456")
            .click(this.submitButton!)
            .pause(2000)
    })

    it("should cast a ballot", (browser) => {
        // navigate to the election list
        const electionListLabel = browser.element.findByText("Election List")
        browser.assert.visible(electionListLabel)
        const isOpenLabel = browser.element.findByText("OPEN")
        browser.assert.visible(isOpenLabel)
        browser.assert.visible("#click-to-vote-button").click("#click-to-vote-button")
        browser.pause(500)
        // navigate to ballot instructions
        const ballotInstructionsLabel = browser.element.findByText("Instructions")
        browser.assert.visible(ballotInstructionsLabel)
        browser.assert.visible("#start-voting-button").click("#start-voting-button")
        browser.pause(500)
        // navigate to ballot casting
        browser.assert.visible("#candidate-0-input").click("#candidate-0-input")
        browser.assert.visible("#next-button").click("#next-button")
        browser.pause(500)
        // navigate to ballot review
        const reviewLabel = browser.element.findByText("Review your ballot")
        browser.assert.visible(reviewLabel)
        browser.assert.visible("#cast-ballot-button").click("#cast-ballot-button")
        browser.pause(500)
        // navigate to end of ballot casting
    })

    // this.it('should be able to logout', (browser) => {
    //     browser
    //         .url(this.testUrl)
    //         .waitForElementVisible('body', 1000)
    //         .assert.title('Voting Portal')
    //         .assert.visible('input[name=username]')
    //         .setValue('input[name=username]', this.username)
    //         .assert.visible('input[name=password]')
    //         .setValue('input[name=password]', this.password)
    //         .assert.visible(this.submitButton)
    //         .click(this.submitButton)
    //         .pause(1000)
    //         .assert.urlEquals(browser.globals.devServerURL + '/dashboard')
    //         .assert.visible('#logout-button')
    //         .click('#logout-button')
    //         .pause(1000)
    //         .assert.urlEquals(browser.globals.devServerURL + '/login')
    //         .end();
    // });
})
