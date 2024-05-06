// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/* eslint-disable testing-library/await-async-query */
import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
}

const getRandomIntInclusive = (min: number, max: number) => {
    min = Math.ceil(min)
    max = Math.floor(max)
    return Math.floor(Math.random() * (max - min + 1)) + min
}

// eslint-disable-next-line jest/valid-describe-callback
describe("cast ballot", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl =
        "http://127.0.0.1:3000/tenant/90505c8a-23a9-4cdf-a26b-4e19f6a097d5/event/0de7ebe5-09ab-4e4b-b228-48153286c648/election-chooser"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    beforeEach(function (this: ExtendDescribeThis<LoginThis>, browser: NightwatchAPI) {
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
    })

    afterEach(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser
            .click("button.profile-menu-button")
            .click("li.logout-button")
            .click("button.ok-button")
            .end()
    })

    it("should cast a ballot", async (browser: NightwatchAPI) => {
        let selectedElectionText = ""
        let selectedCandidateText = ""

        // navigate to the election list
        const electionListLabel = browser.element.findByText("Election List")
        browser.assert.visible(electionListLabel)

        browser.assert.visible("div.election-item")
        const electionCount = await browser.element.findAll("div.election-item").count()
        const selectedElection = getRandomIntInclusive(1, electionCount)

        await browser.getText(
            `div.elections-list div.election-item:nth-child(${selectedElection}) p.election-title`,
            (result) => {
                selectedElectionText = result.value as string
            }
        )

        const electionSelector = `div.elections-list div.election-item:nth-child(${selectedElection}) button.click-to-vote-button`
        browser.assert.visible(electionSelector).click(electionSelector)
        browser.pause(500)

        // navigate to ballot instructions
        const ballotInstructionsLabel = browser.element.findByText("Instructions")
        browser.assert.visible(ballotInstructionsLabel)
        browser.assert.visible("button.start-voting-button").click("button.start-voting-button")
        browser.pause(500)

        // navigate to ballot casting
        const electionLabel = browser.element.findByText(selectedElectionText)
        browser.assert.visible(electionLabel)

        const candidateCount = await browser.element.findAll("p.candidate-title").count()
        const selectedCandidate = getRandomIntInclusive(1, candidateCount)

        await browser.getText(
            `div.candidates-list div.candidate-item:nth-child(${selectedCandidate}) p.candidate-title`,
            (result) => {
                selectedCandidateText = result.value as string
            }
        )

        const candidateSelector = `div.candidates-list div.candidate-item:nth-child(${selectedCandidate}) input.candidate-input`
        browser.click(candidateSelector)
        browser.assert.visible("button.next-button").click("button.next-button")
        browser.pause(500)

        // navigate to ballot review
        const reviewLabel = browser.element.findByText("Review your ballot")
        browser.assert.visible(reviewLabel)
        browser.assert.visible(
            "div.candidates-list div.candidate-item:nth-child(1) p.candidate-title"
        )
        browser.assert.textEquals(
            "div.candidates-list div.candidate-item:nth-child(1) p.candidate-title",
            selectedCandidateText
        )
        browser.assert.visible("button.cast-ballot-button").click("button.cast-ballot-button")
        browser.pause(500)

        // navigate to end of ballot casting
        const castLabel = browser.element.findByText("Your vote has been cast")
        browser.assert.visible(castLabel)

        // we do not push the finish button bacause it will navigate out and cannot logout then
        browser.assert.visible("button.finish-button")
        // browser.assert.visible("button.finish-button").click("button.finish-button")
        // browser.pause(500)
        // browser.assert.urlContains("sequentech")
    })
})
