import {admin_portal_password, admin_portal_username, testUrl} from ".."

exports.command = function (username = admin_portal_username, password = admin_portal_password) {
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    this.window
        .maximize()
        .navigateTo(testUrl)
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
