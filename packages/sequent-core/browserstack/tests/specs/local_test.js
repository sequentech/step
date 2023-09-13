// SPDX-FileCopyrightText: 2022 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
describe('wasm test local', () => {
  it('tests pass', async () => {
    await browser.url("http://localhost:8080/src/wasm/test/test.html");
    await browser.waitUntil(
      async () => (await browser.getTitle()).match("sequent-core wasm test"),
      20000,
      "Title didn't match"
    );

    const lognew = await $('#lognew');

    await browser.waitUntil(
      async () => (await lognew.getText()).startsWith("hash: "),
      { 
        timeout: 20000,
        timeoutMsg: 'Get text timeout' 
      }
    );
  
  });
});
