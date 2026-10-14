// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// Real administration portal against the owned E2E network. API calls and
// trustee proofs are never mocked; the saved browser download is reused at tally.
import { createServer } from "node:http";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import {
  chromium,
  type BrowserContext,
  type Page,
} from "../../packages/node_modules/playwright/index.mjs";
import { expect } from "../../packages/node_modules/@playwright/test/index.mjs";
import { verifyPublicTranscript } from "./public-transcripts.mjs";
import { serveDist } from "../../packages/ui-test-kit/server/static.ts";

interface Input {
  name: string;
  password: string;
  eventId: string;
  ceremonyId: string;
  tallyId?: string;
  otp?: string;
}
interface Client {
  context: BrowserContext;
  page: Page;
  filename: string;
  file: Buffer;
}
const output = process.env.STEP_E2E_OUTPUT_DIR ?? "/out";
const tenant = process.env.SUPER_ADMIN_TENANT_ID;
if (!tenant) throw new Error("SUPER_ADMIN_TENANT_ID is required");
await mkdir(output, { recursive: true });
const settings = JSON.parse(
  await readFile("packages/admin-portal/dist/global-settings.json", "utf8"),
);
Object.assign(settings, {
  DEFAULT_TENANT_ID: tenant,
  KEYCLOAK_URL: "http://keycloak:8090/",
  HASURA_URL: "http://graphql-engine:8080/v1/graphql",
  PUBLIC_BUCKET_URL: "http://minio:9000/public/",
  QUERY_POLL_INTERVAL_MS: 2000,
  QUERY_FAST_POLL_INTERVAL_MS: 500,
});
const portal = await serveDist(
  resolve("packages/admin-portal/dist"),
  new Map([["global-settings.json", JSON.stringify(settings)]]),
);
// A second static origin avoids the portal's authentication redirect for the
// anonymous public observer while serving the same production WASM assets.
const observer = await serveDist(
  resolve("packages/admin-portal/dist"),
  new Map([
    ["index.html", "<!doctype html><title>Public transcript observer</title>"],
  ]),
);
const browser = await chromium.launch({ headless: true });
const clients = new Map<string, Client>();
const failures: string[] = [];

async function open(
  input: Input,
): Promise<{ context: BrowserContext; page: Page }> {
  const context = await browser.newContext({
    locale: "en-US",
    viewport: { width: 1440, height: 1000 },
  });
  await context.route(/^https?:\/\/harvest(?::\d+)?\//, async (route) => {
    failures.push("Browser attempted to call internal Harvest directly");
    await route.abort();
  });
  const page = await context.newPage();
  page.on("pageerror", (error) => failures.push(error.message));
  await page.goto(
    `${portal.origin}/sequent_backend_election_event/${input.eventId}`,
  );
  await page.locator('input[name="username"]').fill(input.name);
  await page.locator('input[name="password"]').fill(input.password);
  await page
    .locator('button[type="submit"], input[type="submit"]')
    .first()
    .click();
  // The existing synthetic OTP configuration is enabled by the driver only
  // while this real authorization-code login runs, then restored.
  if (input.otp) {
    await page.waitForFunction(
      (origin) =>
        window.location.origin === origin ||
        Boolean(document.querySelector('input[name="otp1"]')),
      portal.origin,
      { timeout: 30000 },
    );
    const otp = page.locator('input[name="otp1"]');
    if (await otp.isVisible()) {
      for (let index = 0; index < input.otp.length; index++) {
        await page
          .locator(`input[name="otp${index + 1}"]`)
          .fill(input.otp[index]);
      }
      await page
        .locator('button[type="submit"], input[type="submit"]')
        .first()
        .click();
    }
  }
  await page.waitForURL((url) => url.origin === portal.origin, {
    timeout: 60000,
  });
  return { context, page };
}

async function submitFile(
  page: Page,
  client: Client,
  action: "ConfirmKeyCustody" | "AuthorizeTallyCustody",
) {
  const response = page.waitForResponse(
    (response) => {
      if (
        !response.url().includes("/v1/graphql") ||
        response.request().method() !== "POST"
      )
        return false;
      return response.request().postDataJSON()?.operationName === action;
    },
    { timeout: 130000 },
  );
  await page.locator('input[type="file"]').setInputFiles({
    name: client.filename,
    mimeType: "application/json",
    buffer: client.file,
  });
  const recorded = await response;
  const reply = await recorded.json();
  expect(reply.errors).toBeUndefined();
  expect(Object.values(reply.data)[0]).toMatchObject({ is_valid: true });
  const posted = recorded.request().postDataJSON().variables as {
    custodyProofJson: string;
  };
  expect((await recorded.request().allHeaders())["x-hasura-role"]).toBe(
    "trustee-ceremony",
  );
  expect(JSON.parse(posted.custodyProofJson).request.trustee_name).toBe(
    JSON.parse(client.file.toString()).trustee_name,
  );
  return posted.custodyProofJson;
}

async function keyCustody(input: Input) {
  if (clients.has(input.name)) throw new Error("Duplicate browser trustee");
  const { context, page } = await open(input);
  await page.getByRole("tab", { name: "Keys", exact: true }).click();
  await page.locator(".keys-view-trustee-icon").first().click();
  await page.getByRole("button", { name: "Next", exact: true }).click();
  const pending = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Download Ceremony File", exact: true })
    .click();
  const downloaded = await pending;
  const filename = downloaded.suggestedFilename();
  expect(filename).toBe(`${input.name}-custody-request.json`);
  const path = resolve(output, filename);
  await downloaded.saveAs(path);
  const file = await readFile(path);
  expect(JSON.parse(file.toString())).toMatchObject({
    keys_ceremony_id: input.ceremonyId,
    election_event_id: input.eventId,
    tally_session_id: null,
  });
  await writeFile(resolve(output, `${input.name}-ceremony-copy.json`), file);
  await page.getByRole("button", { name: "Next", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("checkbox", { name: "First copy saved" }).check();
  await dialog.getByRole("checkbox", { name: "Second copy saved" }).check();
  await dialog
    .getByRole("button", { name: "Confirm Saved Copies and Continue" })
    .click();
  await expect(
    page.getByRole("heading", { name: "Confirm Trustee Custody" }),
  ).toBeVisible();
  // A rejected public file must show the actual validation error, then allow
  // the operator to choose the genuine download and continue.
  const wrong = {
    ...JSON.parse(file.toString()),
    trustee_name: "another-trustee",
  };
  await page.locator('input[type="file"]').setInputFiles({
    name: "wrong-ceremony.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(wrong)),
  });
  await expect(
    page.getByText(/does not match this trustee and key ceremony/),
  ).toBeVisible({ timeout: 10000 });
  await expect(
    page.getByRole("button", { name: "Next", exact: true }),
  ).toBeDisabled();
  const client = { context, page, filename, file };
  const proof = await submitFile(page, client, "ConfirmKeyCustody");
  clients.set(input.name, client);
  await page.screenshot({
    path: resolve(output, `${input.name}-key-custody.png`),
    fullPage: true,
  });
  return { file: file.toString(), proof, filename };
}

async function tallyCustody(input: Input, recover = false) {
  const client = clients.get(input.name);
  if (!client) throw new Error("The saved key-ceremony file is missing");
  const { page } = client;
  await page.goto(
    `${portal.origin}/sequent_backend_election_event/${input.eventId}`,
  );
  await page.getByRole("tab", { name: "Tally", exact: true }).click();
  await page.getByLabel("Add Tally Key", { exact: true }).click();
  await expect(
    page.getByText(
      "Upload your saved ceremony file or a signed custody proof:",
      { exact: true },
    ),
  ).toBeVisible();
  await expect(
    page.getByText(/Upload the ceremony file saved during the key ceremony/),
  ).toBeVisible();
  await expect(
    page.getByText("Supported format: JSON", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Download Ceremony File", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Next", exact: true }),
  ).toBeDisabled();
  if (recover) {
    const savedScope = JSON.parse(client.file.toString());
    expect(savedScope.election_event_id).not.toBe(input.eventId);
    expect(savedScope.keys_ceremony_id).not.toBe(input.ceremonyId);
  }
  const proof = await submitFile(page, client, "AuthorizeTallyCustody");
  const proofScope = JSON.parse(proof).request;
  expect(proofScope.election_event_id).toBe(input.eventId);
  expect(proofScope.keys_ceremony_id).toBe(input.ceremonyId);
  expect(proofScope.tally_session_id).toBe(input.tallyId);
  await page.screenshot({
    path: resolve(output, `${input.name}-tally-custody.png`),
    fullPage: true,
  });
  return { file: client.file.toString(), proof, filename: client.filename };
}

const server = createServer(async (request, response) => {
  try {
    if (request.method === "GET" && request.url === "/health") {
      response.writeHead(200, { "Content-Type": "application/json" }).end(
        JSON.stringify({
          adminOrigin: portal.origin,
          clients: [...clients.keys()],
          failures,
        }),
      );
      return;
    }
    let body = "";
    for await (const chunk of request) {
      body += chunk;
      if (body.length > 10000) throw new Error("Oversized controller request");
    }
    const input: Input = JSON.parse(body);
    const result =
      request.url === "/verify"
        ? await verifyPublicTranscript(browser, observer.origin, input)
        : request.url === "/key-custody"
          ? await keyCustody(input)
          : request.url === "/recover-tally-custody"
            ? await tallyCustody(input, true)
            : request.url === "/tally-custody"
              ? await tallyCustody(input)
              : null;
    if (!result) throw new Error("Unknown controller action");
    response
      .writeHead(200, { "Content-Type": "application/json" })
      .end(JSON.stringify(result));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    failures.push(message);
    for (const [index, context] of browser.contexts().entries()) {
      for (const page of context.pages()) {
        await page
          .screenshot({
            path: resolve(output, `browser-${index}-failure.png`),
            fullPage: true,
          })
          .catch(() => undefined);
        await writeFile(
          resolve(output, `browser-${index}-failure.html`),
          await page.content(),
        ).catch(() => undefined);
      }
    }
    response
      .writeHead(500, { "Content-Type": "application/json" })
      .end(JSON.stringify({ error: message }));
  }
});
server.listen(9230, "0.0.0.0");
for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, async () => {
    await browser.close();
    await portal.close();
    await observer.close();
    server.close(() => process.exit(0));
  });
}
