// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

describe("wbraid ballot verifier smoke test", () => {
    it("verifies a freshly encrypted sample ballot", async () => {
        await browser.url(process.env.BALLOT_VERIFIER_URL || "http://localhost:8080/")
        await browser.waitUntil(async () => (await browser.getTitle()).match("Ballot Verifier"), {
            timeout: 20000,
            timeoutMsg: "Title didn't match",
        })
        await $("button*=Use a sample ballot").click()
        const next = await $("button=Next")
        await browser.waitUntil(async () => next.isEnabled(), {
            timeout: 20000,
            timeoutMsg: "Sample ballot did not pass verification",
        })
        await next.click()
        await $("h5=Verify your ballot selections").waitForDisplayed({timeout: 20000})
    })
})
