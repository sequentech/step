// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {candidateLink, contestLink, electionEventLink, electionLink, pause} from "../.."

interface ConfigItem {
    name: string
    description: string
}
interface Config {
    electionEvent: ConfigItem
    election: ConfigItem
    contest: ConfigItem
    candidate1: ConfigItem
    candidate2: ConfigItem
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

export const createElectionEvent: Election = {
    config: {
        electionEvent: {
            name: "this is a test election event name",
            description: "this is a test election event description",
        },
        election: {
            name: "this is a test election name",
            description: "this is a test election description",
        },
        contest: {
            name: "this is a test contest name",
            description: "this is a test contest description",
        },
        candidate1: {
            name: "this is candidate one name",
            description: "this is candidate one description",
        },
        candidate2: {
            name: "this is candidate two name",
            description: "this is candidate two description",
        },
    },
    createElectionEvent: function (this: Election, browser: NightwatchAPI): void {
        // create an election event"
        browser.assert.urlContains(electionEventLink)
        browser.assert.visible(`a.${electionEventLink!}`).element(`a.${electionEventLink!}`).click()
        browser.assert
            .visible("input[name=name]")
            .element("input[name=name]")
            .sendKeys(this?.config?.electionEvent.name)
        browser.assert
            .visible("input[name=description]")
            .element("input[name=description]")
            .sendKeys(this?.config?.electionEvent.description)
        browser.assert
            .enabled(`button.election-event-save-button`)
            .element("button.election-event-save-button")
            .click()
    },
    createElection: function (this: Election, browser: NightwatchAPI): void {
        // create an election"
        browser.assert.urlContains(electionEventLink)
        browser.assert.visible(`a.${electionLink!}`).element(`a.${electionLink!}`).click()
        browser.assert
            .visible("input[name=name]")
            .element("input[name=name]")
            .sendKeys(this?.config?.election.name)
        browser.assert
            .visible("input[name=description]")
            .element("input[name=description]")
            .sendKeys(this?.config?.election.description)
        browser.element("button.election-save-button").click()
    },
    createContest: function (this: Election, browser: NightwatchAPI): void {
        // create a contest"
        browser.assert.urlContains(electionLink)
        browser.assert.visible(`a.${contestLink!}`).element(`a.${contestLink!}`).click()
        browser.assert
            .visible("input[name=name]")
            .element("input[name=name]")
            .sendKeys(this?.config?.contest.name)
        browser.assert
            .visible("input[name=description]")
            .element("input[name=description]")
            .sendKeys(this?.config?.contest.description)
        browser.element("button.contest-save-button").click()
    },
    createCandidates: function (this: Election, browser: NightwatchAPI): void {
        browser.assert.urlContains(contestLink)

        // create a candidate one"
        browser.assert.visible(`a.${candidateLink!}`).element(`a.${candidateLink!}`).click()
        browser.assert
            .visible("input[name=name]")
            .element("input[name=name]")
            .sendKeys(this?.config?.candidate1.name)
        browser.assert
            .visible("input[name=description]")
            .element("input[name=description]")
            .sendKeys(this?.config?.candidate1.description)
        browser.assert
            .enabled("button.candidate-save-button")
            .element("button.candidate-save-button")
            .click()

        // create a candidate two"
        browser.pause(pause.short)
        browser.assert.visible(`a.${candidateLink!}`).element(`a.${candidateLink!}`).click()
        browser.assert
            .visible("input[name=name]")
            .element("input[name=name]")
            .sendKeys(this?.config?.candidate2.name)
        browser.assert
            .visible("input[name=description]")
            .element("input[name=description]")
            .sendKeys(this?.config?.candidate2.description)
        browser.assert
            .enabled("button.candidate-save-button")
            .element("button.candidate-save-button")
            .click()
    },
}
