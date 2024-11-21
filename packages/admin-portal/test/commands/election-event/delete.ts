// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {candidateLink, contestLink, electionEventLink, electionLink} from "../.."
import {createElectionEvent} from "./create"
import {pause} from "../../index"

export const deleteElectionEvent = {
    deleteElectionEvent: (browser: NightwatchAPI) => {
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.electionEvent.name}']`,
            clickElement: `//a//span[text()='${createElectionEvent.config.electionEvent.name}']/../../..//div[contains(@class,'menu-actions-sequent')]//*[local-name()='svg']`,
        })
        // 	browser.useCss().pause(pause.short)
        // 	browser
        // .useXpath()
        // .waitForElementVisible(`//li[contains(@class, 'menu-action-archive-${electionEventLink!}')]`, 10000)
        // .click(`//li[contains(@class, 'menu-action-archive-${electionEventLink!}')]`)
        // .useCss();
        browser.useCss().assert.visible(`li.menu-action-archive-${electionEventLink!}`)
        browser.useCss().element(`li.menu-action-archive-${electionEventLink!}`).click()
        browser.element(`button.ok-button`).click()
    },
    deleteElection: (browser: NightwatchAPI) => {
        // delete election
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.election.name}']`,
            clickElement: `//a//p[normalize-space()='${createElectionEvent.config.election.name}']/../..//div[contains(@class,'menu-actions-sequent')]//*[local-name()='svg']`,
        })
        browser
            .useCss()
            .assert.visible(`li.menu-action-delete-${electionLink!}`)
            .click(`li.menu-action-delete-${electionLink!}`)
        browser.assert.enabled(`button.ok-button`).click(`button.ok-button`)
        browser.assert.not.visible(
            `a[title='${
                createElectionEvent.config.election.name
            }'] + div.menu-actions-${electionLink!}`
        )
    },
    deleteContest: (browser: NightwatchAPI) => {
        // delete contest
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.contest.name}']`,
            clickElement: `//a//p[normalize-space()='${createElectionEvent.config.contest.name}']/../..//div[contains(@class,'menu-actions-sequent')]//*[local-name()='svg']`,
        })
        browser
            .useCss()
            .assert.visible(`li.menu-action-delete-${contestLink!}`)
            .element(`li.menu-action-delete-${contestLink!}`)
            .click()
        browser.assert.enabled(`button.ok-button`).element(`button.ok-button`).click()
        browser.assert.not.visible(
            `a[title='${
                createElectionEvent.config.contest.name
            }'] + div.menu-actions-${contestLink!}`
        )
    },
    deleteCandidates: (browser: NightwatchAPI) => {
        // delete candidate two
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.candidate2.name}']`,
            clickElement: `//a//p[normalize-space()='${createElectionEvent.config.candidate2.name}']/../..//div[contains(@class,'menu-actions-sequent')]//*[local-name()='svg']`,
        })
        browser
            .useCss()
            .assert.visible(`li.menu-action-delete-${candidateLink!}`)
            .element(`li.menu-action-delete-${candidateLink!}`)
            .click()
        browser
            .useCss()
            .assert.visible(`button.ok-button`)
            .assert.enabled(`button.ok-button`)
            .click(`button.ok-button`)
        browser
            .useCss()
            .assert.not.visible(
                `a[title='${
                    createElectionEvent.config.candidate2.name
                }'] + div.menu-actions-${candidateLink!}`
            )

        // delete candidate one
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.candidate1.name}']`,
            clickElement: `//a//p[normalize-space()='${createElectionEvent.config.candidate1.name}']/../..//div[contains(@class,'menu-actions-sequent')]//*[local-name()='svg']`,
        })
        browser
            .useCss()
            .assert.visible(`li.menu-action-delete-${candidateLink!}`)
            .element(`li.menu-action-delete-${candidateLink!}`)
            .click()
        browser.useCss().assert.visible(`button.ok-button`).click(`button.ok-button`)
        browser
            .useCss()
            .assert.not.visible(
                `a[title='${
                    createElectionEvent.config.candidate1.name
                }'] + div.menu-actions-${candidateLink!}`
            )
    },
}
