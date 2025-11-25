// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {NightwatchAPI} from "nightwatch"
import {pause} from "../../../admin-portal/test"

type VerifyConfigType = {
    ballotPath: string
    ballotId: string
}

export const verifyBallot = (browser: NightwatchAPI, {ballotPath, ballotId}: VerifyConfigType) => {
    browser
        .waitForElementVisible("body")
        .useXpath()
        .assert.visible(`//span[normalize-space()="Step 1: Import your ballot"]`)
    browser.uploadFile(`//input[contains(@class,'drop-input-file')]`, ballotPath)
    browser.sendKeys(`//input[@id=":rb:"]`, ballotId)
    browser.click(`//button/span[normalize-space()='Next']`)
    browser.assert.not
        .elementPresent(`//p[normalize-space()="Does’t match the decoded ballot ID"]`)
        .pause(pause.medium)
}
