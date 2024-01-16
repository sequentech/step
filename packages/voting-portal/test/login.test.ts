import {ExtendDescribeThis} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("login", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl =
        "http://127.0.0.1:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/5960217c-ac34-40b2-99ae-40ecc54f03f9"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    beforeEach(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.navigateTo(this.testUrl!)
    })

    it("should be able to login", (browser) => {
        browser
            .waitForElementVisible(this.username!)
            .waitForElementVisible(this.password!)
            .assert.visible("input[name=username]")
            .sendKeys(this.username!, "felix")
            .assert.visible("input[name=password]")
            .sendKeys(this.password!, "felix")
            .assert.visible(this.submitButton!)
            .click(this.submitButton!)
            .pause(2000)
            .assert.visible("input[name=code]")
            .sendKeys("#code", "123456")
            .assert.visible(this.submitButton!)
            .click(this.submitButton!)
            .pause(1000)
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

