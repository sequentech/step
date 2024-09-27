// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"

describe("login", function () {
    before(function (browser) {
        browser.login()
    })

    after(function (browser) {
        browser.logout().end()
    })

    it("create an election event", (browser: NightwatchAPI) =>
        createElectionEvent.createElectionEvent(browser))
    it("create an election", (browser: NightwatchAPI) =>
        createElectionEvent.createElection(browser))
    it("create a contest", (browser: NightwatchAPI) => createElectionEvent.createContest(browser))
    it("create candidates", (browser: NightwatchAPI) =>
        createElectionEvent.createCandidates(browser))

    // it("delete candidates", (browser: NightwatchAPI) =>
    //     deleteElectionEvent.deleteCandidates(browser))
    // it("delete contest", (browser: NightwatchAPI) => deleteElectionEvent.deleteContest(browser))
    // it("delete an election", (browser: NightwatchAPI) =>
    //     deleteElectionEvent.deleteElection(browser))
    it("delete an election event", (browser: NightwatchAPI) =>
        deleteElectionEvent.deleteElectionEvent(browser))
})
