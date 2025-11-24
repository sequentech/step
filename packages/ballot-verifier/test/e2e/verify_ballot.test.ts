// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {loginUrl, password, pause, username} from ".."
import {verifyBallot} from "../commands/verifyBallot"

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

    // running from voting-portal/verify_ballot.test.ts worls better since it ends up eexporting the ballot file and getting the ballotId a lot more dynamically
    it("should be able to verify ballot", async (browser: NightwatchAPI) => {
        verifyBallot(browser, {
            ballotPath: `${require("path").resolve(
                __dirname + "/downloads/"
            )}${"/a65f2520-9003-40ae-b1b3-7683c1f51dbb"}-ballot.txt`,
            ballotId: "dc2c427b3607dea9561a8358b9ac9d6aef387817cd11d7341629a2a0fe96d673",
        })
    })
})
