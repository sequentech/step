// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const { JSDOM } = require("jsdom");

const script = readFileSync(
  join(
    __dirname,
    "../../main/resources/theme/sequent.admin-portal/login/resources/js/enrollment-window.js",
  ),
  "utf8",
);

// The markup register.ftl renders: the embassy select, one notice per Post,
// and Continue, which the termsOfService checkbox also enables and disables.
const setup = (selected) => {
  const dom = new JSDOM(
    `<!doctype html>
      <form id="kc-register-form">
        <select id="country" name="country">
          <option value="UAE/Dubai PCG">UAE</option>
          <option value="Spain/Canary office">Spain</option>
        </select>
        <select id="embassy" name="embassy">
          <option value="Dubai PCG" ${selected === "Dubai PCG" ? "selected" : ""}>Dubai PCG</option>
          <option value="Canary office" ${selected === "Canary office" ? "selected" : ""}>Canary office</option>
          <option value="Rome PE" ${selected === "Rome PE" ? "selected" : ""}>Rome PE</option>
          <option value="Oslo PE" ${selected === "Oslo PE" ? "selected" : ""}>Oslo PE</option>
          <option value="Bern PE" ${selected === "Bern PE" ? "selected" : ""}>Bern PE</option>
        </select>
        <div data-enrollment-embassy="Dubai PCG" data-enrollment-state="before" hidden>Opens on Feb 9</div>
        <div data-enrollment-embassy="Canary office" data-enrollment-state="open" hidden>Open until Mar 31</div>
        <div data-enrollment-embassy="Rome PE" data-enrollment-state="closed" hidden>Not open now</div>
        <div data-enrollment-embassy="Bern PE" data-enrollment-state="not-configured" hidden>Can't be checked</div>
        <input id="termsOfServiceText" type="submit" value="Continue">
      </form>`,
    { runScripts: "outside-only", url: "https://example.test" },
  );
  dom.window.eval(script);
  const document = dom.window.document;
  return {
    dom,
    document,
    form: document.getElementById("kc-register-form"),
    embassy: document.getElementById("embassy"),
    submit: document.getElementById("termsOfServiceText"),
    notice: (post) => document.querySelector(`[data-enrollment-embassy="${post}"]`),
  };
};

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

const submitted = (page) => {
  const event = new page.dom.window.Event("submit", { bubbles: true, cancelable: true });
  return page.form.dispatchEvent(event);
};

test("a Post that hasn't opened shows its opening and holds Continue back", () => {
  const page = setup("Dubai PCG");
  assert.equal(page.notice("Dubai PCG").hidden, false);
  assert.equal(page.notice("Canary office").hidden, true);
  assert.equal(page.submit.getAttribute("aria-disabled"), "true");
  assert.ok(page.submit.classList.contains("enrollment-window-blocked"));
  assert.equal(submitted(page), false);
});

test("an open Post shows its close and lets Continue submit", () => {
  const page = setup("Canary office");
  assert.equal(page.notice("Canary office").hidden, false);
  assert.equal(page.notice("Dubai PCG").hidden, true);
  assert.equal(page.submit.hasAttribute("aria-disabled"), false);
  assert.equal(submitted(page), true);
});

test("a closed Post is held back too", () => {
  const page = setup("Rome PE");
  assert.equal(page.notice("Rome PE").hidden, false);
  assert.equal(submitted(page), false);
});

test("a Post whose window can't be checked is held back", () => {
  const page = setup("Bern PE");
  assert.equal(page.notice("Bern PE").hidden, false);
  assert.equal(submitted(page), false);
});

test("a Post without a window shows nothing and isn't held back", () => {
  const page = setup("Oslo PE");
  for (const post of ["Dubai PCG", "Canary office", "Rome PE"]) {
    assert.equal(page.notice(post).hidden, true);
  }
  assert.equal(submitted(page), true);
});

test("choosing another Post updates the notice and Continue", async () => {
  const page = setup("Dubai PCG");
  page.embassy.value = "Canary office";
  page.embassy.dispatchEvent(new page.dom.window.Event("change", { bubbles: true }));
  await tick();
  assert.equal(page.notice("Canary office").hidden, false);
  assert.equal(page.notice("Dubai PCG").hidden, true);
  assert.equal(submitted(page), true);
});

test("a Post picked through the country select is re-read", async () => {
  const page = setup("Canary office");
  // filterSelectAttribute sets the embassy without a change event on it.
  page.embassy.value = "Dubai PCG";
  page.document
    .getElementById("country")
    .dispatchEvent(new page.dom.window.Event("change", { bubbles: true }));
  await tick();
  assert.equal(page.notice("Dubai PCG").hidden, false);
  assert.equal(submitted(page), false);
});

test("the terms checkbox enabling Continue doesn't release a Post that isn't open", () => {
  const page = setup("Dubai PCG");
  page.submit.disabled = false;
  assert.equal(submitted(page), false);
  assert.equal(page.submit.getAttribute("aria-disabled"), "true");
});
