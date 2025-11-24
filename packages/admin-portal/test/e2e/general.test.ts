// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {createElectionEvent} from "../commands/election-event/create"
import {deleteElectionEvent} from "../commands/election-event/delete"

describe("login", function () {
    before(function (browser) {
        browser.login()
    })

    after(async function (browser) {
        // Logout
        browser.logout()
    })

    it("create an election event", async (browser: NightwatchAPI) =>
        createElectionEvent.createElectionEvent(browser))

    it("delete an election event", async (browser: NightwatchAPI) =>
        deleteElectionEvent.deleteElectionEvent(browser))
})
