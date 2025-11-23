// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {loginUrl, password, pause, username} from ".."

describe("login", function () {
    before(function (browser) {
        browser.pause(pause.medium).login({
            loginUrl,
            username,
            password,
        })
    })

    after(function (browser) {
        browser.logout().end()
    })

    it("should be able to login", async (browser: NightwatchAPI) => {
        const electionListLabel = await browser.element.findByText("Ballot List")
        browser.assert.visible(electionListLabel)
    })
})
