// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// An anonymous observer checks the real protocol transcript, including invalid
// anchor and incomplete transcript controls. It has no trustee credentials.
export async function verifyPublicTranscript(browser, origin, input) {
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await page.goto(`${origin}/wbraid-trustee/index.js`);
    return await page.evaluate(async (input) => {
      const module = await import(
        new URL("/wbraid-trustee/index.js", window.location.origin).href
      );
      await module.default({
        module_or_path: new URL(
          "/wbraid-trustee/index_bg.wasm",
          window.location.origin,
        ).href,
      });
      await module.initThreadPool(
        Math.min(navigator.hardwareConcurrency || 2, 4),
      );
      const verifier = module.WasmVerifier;
      const anchors = {
        ...input.anchors,
        configuration_hash: verifier.configurationHash(
          input.configurationMessage,
        ),
        ballots_hash: input.ballotsMessage
          ? verifier.ballotsHash(input.ballotsMessage)
          : null,
      };
      const transcript = await verifier.fetchTranscript(
        input.boardUrl,
        input.dkgBoard,
        input.tallyBoard,
      );
      const report = JSON.parse(
        verifier.verifyTranscript(transcript, JSON.stringify(anchors)),
      );
      const wrongAnchor = JSON.parse(
        verifier.verifyTranscript(
          transcript,
          JSON.stringify({
            ...anchors,
            configuration_hash: "00".repeat(64),
          }),
        ),
      );
      const incomplete = JSON.parse(transcript);
      if (incomplete.tally_messages) incomplete.tally_messages = [];
      else incomplete.dkg_messages = [];
      const truncated = JSON.parse(
        verifier.verifyTranscript(
          JSON.stringify(incomplete),
          JSON.stringify(anchors),
        ),
      );
      return {
        transcript: JSON.parse(transcript),
        anchors,
        report,
        wrongAnchor,
        truncated,
      };
    }, input);
  } finally {
    await context.close();
  }
}
