// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
        const electionListLabel = await browser.element.findByText("Import your ballot")
        browser.assert.visible(electionListLabel)
    })
})
