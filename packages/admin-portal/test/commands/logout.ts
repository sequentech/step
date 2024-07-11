exports.command = function () {
    this.click("header [data-testid='AccountCircleIcon']")
        .click("li.logout-button")
        .click("button.ok-button")
        .pause(2000)

    return this
}
