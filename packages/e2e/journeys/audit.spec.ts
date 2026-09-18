// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { test, expect } from "../fixtures";
import { login, prepareBallot } from "../../voting-portal/test/load/flow";

test("a fresh browser ballot can be audited in the verifier", async ({
  page,
  fixture,
}, info) => {
  await page.goto(fixture.loginUrl);
  await login(page, {
    username: `${fixture.usernamePrefix}4`,
    password: fixture.password,
  });
  await expect(page.locator(".election-item").first()).toBeVisible();
  await prepareBallot(page, 0);
  await page.getByRole("button", { name: "Audit ballot", exact: true }).click();
  await page
    .getByRole("button", {
      name: "Yes, I want to DISCARD my ballot to audit it",
    })
    .click();
  const hash = page.locator(".hash-text").first();
  await expect(hash).toContainText("Your Ballot ID:");
  const ballotId = (await hash.innerText())
    .split(":")
    .slice(1)
    .join(":")
    .trim();
  expect(ballotId.length).toBeGreaterThan(20);
  const downloaded = page.waitForEvent("download");
  await page.getByRole("button", { name: /download/i }).click();
  const ballot = await downloaded;
  const file = info.outputPath("auditable-ballot.txt");
  await ballot.saveAs(file);
  await page.goto(fixture.verifierUrl);
  await page.locator('input[type="file"]').setInputFiles(file);
  await page.getByLabel("Ballot ID", { exact: true }).fill(ballotId);
  const next = page.getByRole("button", { name: "Next", exact: true });
  await expect(next).toBeEnabled();
  await next.click();
  await expect(page).toHaveURL(/confirmation/);
});
