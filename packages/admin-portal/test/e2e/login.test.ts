// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExtendDescribeThis, NightwatchAPI} from "nightwatch"

interface LoginThis {
    testUrl: string
    username: string
    password: string
    submitButton: string
}

// eslint-disable-next-line jest/valid-describe-callback
describe("login", function (this: ExtendDescribeThis<LoginThis>) {
    this.testUrl = "http://127.0.0.1:3002"
    this.username = "input[name=username]"
    this.password = "input[name=password]"
    this.submitButton = "*[type=submit]"

    before(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser.navigateTo(this.testUrl!)
    })

    after(function (this: ExtendDescribeThis<LoginThis>, browser) {
        browser
			//ts ignores to be removed
            //@ts-ignore
            .logout()
            .end()
    })

    it("should be able to login", async (browser: NightwatchAPI) => {
        browser
            .navigateTo(this.testUrl!)
            //@ts-ignore
            .login("admin", "admin")
        browser.assert.urlContains("sequent_backend_election_event")
    })
})
