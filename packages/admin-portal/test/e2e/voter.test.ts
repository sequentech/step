import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"
import {voterDetails} from ".."
import {assertListItemText} from "../commands/assertListItemText"
import { createElectionEvent } from "../commands/election-event/create"
import { deleteElectionEvent } from "../commands/election-event/delete"

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
    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.login()

        // create election event
        createElectionEvent.createElectionEvent(browser)
        createElectionEvent.createElection(browser)
        createElectionEvent.createContest(browser)
        createElectionEvent.createCandidates(browser)
    })

    after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
        //delete election event
        deleteElectionEvent.deleteCandidates(browser)
        deleteElectionEvent.deleteContest(browser)
        deleteElectionEvent.deleteElection(browser)
        deleteElectionEvent.deleteElectionEvent(browser)

        // Logout
        browser.logout()
    })

    it("create a voter", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEventName}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.assert
                        .visible("button.voter-add-button")
                        .click("button.voter-add-button")
                } else {
                    browser.assert.visible("button.add-button").click("button.add-button")
                }

                browser
                    .sendKeys("input[name=first_name]", voterDetails.firstName)
                    .sendKeys("input[name=last_name]", voterDetails.lastName)
                    .sendKeys("input[name=email]", voterDetails.email)
                    .sendKeys("input[name=username]", voterDetails.username)
                    .assert.enabled("button[type=submit]")
                    .click("button[type=submit]")
                    .pause(1000)
                // .debug()

                // const fnameEl = await browser.element.findAll(
                // 	"span.first_name"
                // )
                // const lnameEl = await browser.element.findAll(
                // 	"span.last_name"
                // )
                // const emailEl = await browser.element.findAll(
                // 	"span.email"
                // )
                // const usernameEl = await browser.element.findAll(
                // 	"span.username"
                // )
                // browser.assert.textContains(fnameEl[fnameEl.length - 1], voterDetails.firstName)
                // 	.assert.textContains(lnameEl[lnameEl.length - 1], voterDetails.lastName)
                // 	.assert.textContains(emailEl[emailEl.length - 1], voterDetails.email)
                // 	.assert.textContains(usernameEl[usernameEl.length - 1], voterDetails.email)
                Promise.all([
                    assertListItemText({
                        el: "span.first_name",
                        text: voterDetails.firstName,
                        browser,
                    }),
                    assertListItemText({
                        el: "span.last_name",
                        text: voterDetails.lastName,
                        browser,
                    }),
                    assertListItemText({
                        el: "span.email",
                        text: voterDetails.email,
                        browser,
                    }),
                    assertListItemText({
                        el: "span.username",
                        text: voterDetails.email,
                        browser,
                    }),
                ])
            }
        )
    })

    it("edit a voter to set password", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEventName}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
                    browser
                        .sendKeys("input[name=password]", "secretepassword")
                        .sendKeys("input[name=repeat_password]", "secretepassword")
                        .assert.enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(200)

                    await assertListItemText({
                        el: "span.first_name",
                        text: voterDetails.firstName,
                        browser,
                    })
                }
            }
        )
    })

    it("edit a voter to set area", async (browser: NightwatchAPI) => {
        // create area
        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.assert.visible("button.area-add-button").click("button.area-add-button")
                } else {
                    browser.assert.visible("button.add-button").click("button.add-button")
                }
                browser
                    .sendKeys("input[name=name]", "this is an area name")
                    .assert.enabled("button[type=submit]")
                    .click("button[type=submit]")
                    .pause(200)

                await assertListItemText({
                    el: "span.area-name",
                    text: "this is an area name",
                    browser,
                })
            }
        )

        // activate voters tab
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEventName}']`
        )
        resultElement[resultElement.length - 1].click()

        browser.assert.visible("a.election-event-voter-tab").click("a.election-event-voter-tab")

        browser.isPresent(
            {
                selector: "button.voter-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            async (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".edit-voter-icon").click(".edit-voter-icon")
                    browser.assert.visible(".select-voter-area").click(".select-voter-area")
                    const opcion = await browser.element.findByRole("option")
                    opcion.click()
                    browser.assert
                        .enabled("button[type=submit]")
                        .click("button[type=submit]")
                        .pause(200)

                    await assertListItemText({
                        el: "span.first_name",
                        text: voterDetails.firstName,
                        browser,
                    })

                    // .assert.textContains("span.first_name", voterDetails.firstName)
                }
            }
        )

        // delete area
        browser.assert.visible("a.election-event-area-tab").click("a.election-event-area-tab")

        browser.isPresent(
            {
                selector: "button.area-add-button",
                suppressNotFoundErrors: true,
                timeout: 1000,
            },
            (result) => {
                if (result.value) {
                    browser.end()
                } else {
                    browser.assert.visible(".delete-area-icon").click(".delete-area-icon")
                    browser.assert
                        .enabled(`button.ok-button`)
                        .click("button.ok-button")
                        .pause(1000)
                        .assert.not.elementPresent("span.area-description")
                }
            }
        )
    })

    it("delete a voter", async (browser: NightwatchAPI) => {
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEventName}']`
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
                    browser.end()
                } else {
                    browser.assert
                        .visible(
                            // `//tr[td/span[contains(@class, 'first_name') and text()=]]/td/button[contains(@class, 'delete-voter-icon')]`
							`//span[normalize-space()=${voterDetails.firstName}]/../../td/button[3]`
                        )
                        .click(
							`//span[normalize-space()=${voterDetails.firstName}]/../../td/button[3]`
                        )
                    browser.assert
                        .enabled(`button.ok-button`)
                        .click("button.ok-button")
                        .pause(1000)
                        .assert.not.elementPresent(
                            `//span[contains(@class, 'first_name') and text()=${voterDetails.firstName}]`
                        )
                }
            }
        )
    })
})
