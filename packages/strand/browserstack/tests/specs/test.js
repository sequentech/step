// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
describe("wasm test remote", () => {
  it("tests pass", async () => {
    await browser.url("https://david-test.sequentech.io/test.html");
    await browser.waitUntil(
      async () => (await browser.getTitle()).match("strand wasm test"),
      5000,
      "Title didn't match"
    );

    const lognew = await $('#lognew');

    await browser.waitUntil(
      async () => (await lognew.getText()) === "ok", 
      { 
        timeout: 5000,
        timeoutMsg: 'Get text timeout' 
      }
    );
  
  });
});
