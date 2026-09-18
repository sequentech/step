// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { test as base, expect } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";
import { randomUUID } from "node:crypto";

export interface Fixture {
  tenantId: string;
  eventId: string;
  electionId: string;
  loginUrl: string;
  adminUrl: string;
  verifierUrl: string;
  resultsUrl: string;
  usernamePrefix: string;
  password: string;
  adminUsername: string;
  adminPassword: string;
  auditDsn: string;
}
type CoverageWindow = Window & {
  __coverage__?: Record<string, unknown>;
  __saveE2ECoverage?: (coverage: Record<string, unknown>) => Promise<void>;
};

export const test = base.extend<{ fixture: Fixture; collectCoverage: void }>({
  fixture: async ({}, use) => {
    const file =
      process.env.E2E_FIXTURE ||
      path.join(process.env.E2E_ARTIFACTS || "", "private/fixture.json");
    const fixture = JSON.parse(await fs.readFile(file, "utf8")) as Fixture;
    for (const field of ["tenantId", "eventId", "loginUrl"] as const) {
      if (!fixture[field]) throw new Error(`Fixture is missing ${field}`);
    }
    await use(fixture);
  },
  collectCoverage: [
    async ({ context }, use, info) => {
      if (!process.env.E2E_COVERAGE || process.env.E2E_COVERAGE === "none")
        return use();
      const directory = path.join(
        process.env.E2E_ARTIFACTS!,
        "coverage/frontend/e2e",
      );
      await fs.mkdir(directory, { recursive: true });
      let collected = 0;
      const save = async (coverage: Record<string, unknown> | undefined) => {
        if (!coverage || !Object.keys(coverage).length) return;
        await fs.writeFile(
          path.join(directory, `${randomUUID()}.json`),
          JSON.stringify(coverage),
          { mode: 0o600 },
        );
        collected++;
      };
      await context.exposeBinding(
        "__saveE2ECoverage",
        async (_source, coverage: Record<string, unknown>) => save(coverage),
      );
      await context.addInitScript(() => {
        window.addEventListener("pagehide", () => {
          const instrumented = window as CoverageWindow;
          if (instrumented.__coverage__)
            void instrumented.__saveE2ECoverage?.(instrumented.__coverage__);
        });
      });
      await use();
      for (const page of context.pages()) {
        if (!page.isClosed())
          await save(
            await page
              .evaluate(() => (window as CoverageWindow).__coverage__)
              .catch(() => undefined),
          );
      }
      if (info.status === "passed")
        expect(
          collected,
          "Instrumented test produced no browser coverage",
        ).toBeGreaterThan(0);
    },
    { auto: true },
  ],
});
export { expect };
