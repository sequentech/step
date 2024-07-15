import {candidateLink, contestLink, electionEventLink, electionLink} from ".."

const createElectionEvent = require("./createElectionEvent")

module.exports = {
    deleteElectionEvent: (browser) => {
        // delete election event
        // browser.hoverAndClick(`a[title='${createElectionEvent.config.electionEventName}'] + div.menu-actions-${electionEventLink!}`)
        browser.hoverAndClick({
            hoverElement: `a[title='${createElectionEvent.config.electionEventName}']`,
            clickElement: `a[title='${
                createElectionEvent.config.electionEventName
            }'] + div.menu-actions-${electionEventLink!}`,
        })
        browser.pause(200)
        browser.element(`li.menu-action-delete-${electionEventLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)
    },
    deleteElection: (browser) => {
        // delete election
        browser.hoverAndClick(
            `a[title='${
                createElectionEvent.config.electionName
            }'] + div.menu-actions-${electionLink!}`
        )
        browser.pause(200)
        browser.element(`li.menu-action-delete-${electionLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)
    },
    deleteContest: (browser) => {
        // delete contest
        browser.hoverAndClick(
            `a[title='${
                createElectionEvent.config.contestName
            }'] + div.menu-actions-${contestLink!}`
        )
        browser.pause(200)
        browser.element(`li.menu-action-delete-${contestLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)
    },
    deleteCandidates: async (browser) => {
        // delete candidate two
        browser.hoverAndClick(
            `a[title='${
                createElectionEvent.config.candidate2Name
            }'] + div.menu-actions-${candidateLink!}`
        )
        browser.pause(200)
        browser.element(`li.menu-action-delete-${candidateLink!}`).click()
        browser.element(`button.ok-button`).click()
        browser.pause(200)

        // delete candidate one
        browser.hoverAndClick(
            `a[title='${
                createElectionEvent.config.candidate1Name
            }'] + div.menu-actions-${candidateLink!}`
        )

        browser.element(`li.menu-action-delete-${candidateLink!}`).click()
        browser.pause(200)
        browser.element(`button.ok-button`).click()
        browser.pause(200)
    },
}
