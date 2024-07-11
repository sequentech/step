exports.command = function (username = "admin", password = "admin") {
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.windowRect({width: 1260, height: 890})
        .navigateTo(this.testUrl)
        .waitForElementVisible(this.username)
        .waitForElementVisible(this.password)
        .assert.visible("input[name=username]")
        .sendKeys(this.username, username)
        .assert.visible("input[name=password]")
        .sendKeys(this.password, password)
        .pause(2000)
        .assert.visible(this.submitButton)
        .click(this.submitButton)
        .pause(2000)

    return this
}
