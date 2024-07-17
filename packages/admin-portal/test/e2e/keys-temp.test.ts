import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"
import {electionEventLink} from ".."
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
describe("keys tests", function (this: ExtendDescribeThis<LoginThis>) {
    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        // login
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

    it("has list of keys", async (browser: NightwatchAPI) => {
        await browser.window.maximize()
        // browser.debug()
        const resultElement = await browser.element.findAll(
            `a[title = '${createElectionEvent.config.electionEventName}']`
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
                    //there are no keys so gracefully exit
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
