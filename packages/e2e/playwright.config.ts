// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
import { browserOptions } from "../voting-portal/test/load/browser";

const artifacts =
  process.env.E2E_ARTIFACTS || path.resolve(__dirname, "../../.e2e/manual");
export default defineConfig({
  testMatch: "**/*.spec.ts",
  timeout: 180_000,
  expect: { timeout: 20_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0, // A fresh run owns fresh voters; never repeat a cast after an ambiguous response.
  // Select local scenarios with --grep; container environment forwarding must
  // never allow an accidental focused test to hide the rest of the suite.
  forbidOnly: true,
  globalTimeout: 12 * 60_000,
  outputDir: path.join(artifacts, "private/test-results"),
  reporter: [
    ["line"],
    [
      "html",
      {
        outputFolder: path.join(artifacts, "private/playwright-report"),
        open: "never",
      },
    ],
    ["json", { outputFile: path.join(artifacts, "private/playwright.json") }],
  ],
  use: {
    ...devices["Desktop Chrome"],
    headless: true,
    launchOptions: browserOptions(),
    locale: "en-US",
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [
    "admin-portal",
    "voting-portal",
    "ballot-verifier",
    "results-portal",
  ]
    .map((name) => ({
      name,
      testDir: path.resolve(__dirname, "..", name, "test/e2e"),
    }))
    .concat([{ name: "journeys", testDir: path.join(__dirname, "journeys") }]),
});
