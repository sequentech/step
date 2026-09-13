// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ExtendDescribeThis} from "nightwatch"

interface CustomThis {
    duckDuckGoUrl: string
    searchBox: string
    submitButton: string
}

// callback passed to `describe` should be a regular function (not an arrow function).
describe("duckduckgo example", function (this: ExtendDescribeThis<CustomThis>) {
    const duckDuckGoUrl = "https://duckduckgo.com"
    this.duckDuckGoUrl = duckDuckGoUrl
    const searchBox = "input[name=q]"
    this.searchBox = searchBox
    const submitButton = "*[type=submit]"
    this.submitButton = submitButton

    // callback can be a regular function as well as an arrow function.
    beforeEach(function (this: ExtendDescribeThis<CustomThis>, browser) {
        browser.navigateTo(duckDuckGoUrl)
    })

    // no need to specify `this` parameter when passing an arrow function
    // as callback to `it`.
    it("Search Nightwatch.js and check results", (browser) => {
        browser
            .waitForElementVisible(searchBox)
            .sendKeys(searchBox, ["Nightwatch.js"])
            .click(submitButton)
            .assert.visible("#react-layout")
            .assert.textContains("#react-layout", "Nightwatch.js")
    })
})
