import {NightwatchAPI} from "nightwatch"
import {candidateLink, contestLink, electionEventLink, electionLink} from ".."

interface Config {
    electionEventName: string
    electionName: string
    contestName: string
    candidate1Name: string
    candidate2Name: string
}

interface ElectionMethods {
    createElectionEvent(browser: NightwatchAPI): void
    createElection(browser: NightwatchAPI): void
    createContest(browser: NightwatchAPI): void
    createCandidates(browser: NightwatchAPI): void
}

interface Election extends ElectionMethods {
    config: Config
}

const createElectionEvent: Election = {
    config: {
        electionEventName: "this is a test election event name",
        electionName: "this is a test election name",
        contestName: "this is a test contest name",
        candidate1Name: "this is candidate one name",
        candidate2Name: "this is candidate two name",
    },
    createElectionEvent: function (this: Election, browser: NightwatchAPI): void {
        // create an election event"
        browser.element(`a.${electionEventLink!}`).click()
        browser.element("input[name=name]").sendKeys(this?.config?.electionEventName)
        browser.element("button.election-event-save-button").click()
        browser.pause(5000)
    },
    createElection: function (this: Election, browser: NightwatchAPI): void {
        // create an election"
        browser.element(`a.${electionLink!}`).click()
        browser.element("input[name=name]").sendKeys(this?.config?.electionName)
        browser.element("button.election-save-button").click()
    },
    createContest: function (this: Election, browser: NightwatchAPI): void {
        // create a contest"
        browser.element(`a.${contestLink!}`).click()
        browser.element("input[name=name]").sendKeys(this?.config?.contestName)
        browser.element("button.contest-save-button").click()
    },
    createCandidates: function (this: Election, browser: NightwatchAPI): void {
        // create a candidate one"
        browser.element(`a.${candidateLink!}`).click()
        browser.element("input[name=name]").sendKeys(this?.config?.candidate1Name)
        browser.element("button.candidate-save-button").click()
        browser.pause(500)

        // create a candidate two"
        browser.element(`a.${candidateLink!}`).click()
        browser.element("input[name=name]").sendKeys(this?.config?.candidate2Name)
        browser.element("button.candidate-save-button").click()
        browser.pause(500)
    },
}

module.exports = createElectionEvent
