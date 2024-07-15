import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"
import {electionEventLink} from ".."

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
describe("keys tests", function (this: ExtendDescribeThis<LoginThis>) {
    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        // login
        browser.login()
    })

    after(async function (this: ExtendDescribeThis<LoginThis>, browser) {
        // Logout
        browser.logout()
    })

    it("has list of keys", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
        const resultElement = await browser.element.findAll(`a.menu-item-${electionEventLink!}`)
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
                    browser.assert.visible(".keys-view-admin-icon").click(".keys-view-admin-icon")
                    // browser.waitUntil(async () => {
                    //     const status = await browser
                    //         .element(".keys-ceremony-status > span")
                    //         .getText()
                    //     return status.includes("NOT_STARTED")
                    // })
                    // browser.assert.textContains(".keys-ceremony-status > span", "NOT_STARTED")
                    browser.waitUntil(async () => {
                        const status = await browser
                            .element(".keys-ceremony-status > span")
                            .getText()
                        return status.includes("IN_PROCESS")
                    })
                    browser.assert.textContains(".keys-ceremony-status > span", "IN_PROCESS")
                }
            }
        )
    })
})
