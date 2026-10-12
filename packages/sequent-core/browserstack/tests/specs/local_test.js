// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
describe("wbraid WASM ballot smoke test", () => {
  it("encrypts, hashes and verifies a freshly generated audit opening", async () => {
    await browser.url(
      "http://localhost:8080/sequent-core/src/wasm/test/test.html",
    );
    await browser.waitUntil(
      async () => (await browser.getTitle()).match("sequent-core wasm test"),
      { timeout: 20000, timeoutMsg: "Title didn't match" },
    );

    const lognew = await $("#lognew");

    await browser.waitUntil(
      async () => /^verified: [0-9a-f]{64}$/.test(await lognew.getText()),
      {
        timeout: 20000,
        timeoutMsg: "Get text timeout",
      },
    );
  });
});
