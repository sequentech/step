// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {candidateLink, contestLink, electionEventLink, electionLink} from "../.."
import {createElectionEvent} from "./create"

export const deleteElectionEvent = {
    deleteElectionEvent: (browser: NightwatchAPI) => {
        // delete election event
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.electionEvent.name}']`,
            clickElement: `a[title='${
                createElectionEvent.config.electionEvent.name
            }'] + div.menu-actions-${electionEventLink!}`,
        })
        browser.assert
            .visible(`li.menu-action-delete-${electionEventLink!}`)
            .element(`li.menu-action-delete-${electionEventLink!}`)
            .click()
        browser.element(`button.ok-button`).click()
    },
    deleteElection: (browser: NightwatchAPI) => {
        // delete election
        browser.hoverAndClick(
            `a[title='${
                createElectionEvent.config.election.name
            }'] + div.menu-actions-${electionLink!}`
        )
        browser.assert
            .visible(`li.menu-action-delete-${electionLink!}`)
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
            clickElement: `a[title='${
                createElectionEvent.config.contest.name
            }'] + div.menu-actions-${contestLink!}`,
        })
        browser.assert
            .visible(`li.menu-action-delete-${contestLink!}`)
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
            clickElement: `a[title='${
                createElectionEvent.config.candidate2.name
            }'] + div.menu-actions-${candidateLink!}`,
        })
        browser.assert
            .visible(`li.menu-action-delete-${candidateLink!}`)
            .element(`li.menu-action-delete-${candidateLink!}`)
            .click()
        browser.assert
            .visible(`button.ok-button`)
            .assert.enabled(`button.ok-button`)
            .click(`button.ok-button`)
        browser.assert.not.visible(
            `a[title='${
                createElectionEvent.config.candidate2.name
            }'] + div.menu-actions-${candidateLink!}`
        )

        // delete candidate one
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.candidate1.name}']`,
            clickElement: `a[title='${
                createElectionEvent.config.candidate1.name
            }'] + div.menu-actions-${candidateLink!}`,
        })
        browser.assert
            .visible(`li.menu-action-delete-${candidateLink!}`)
            .element(`li.menu-action-delete-${candidateLink!}`)
            .click()
        browser.assert.visible(`button.ok-button`).click(`button.ok-button`)
        browser.assert.not.visible(
            `a[title='${
                createElectionEvent.config.candidate1.name
            }'] + div.menu-actions-${candidateLink!}`
        )
    },
}
