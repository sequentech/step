// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
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
    "../../main/resources/theme/sequent.admin-portal/login/resources/js/structured-credential.js",
  ),
  "utf8",
);

const setup = ({ pattern = "dddd-dddd", password = "" } = {}) => {
  const dom = new JSDOM(
    `<!doctype html>
      <form id="login">
        <input name="username" aria-invalid="true">
        <label id="pin-label" for="password">PIN</label>
        <div data-structured-credential
             data-credential-pattern="${pattern}"
             data-group-status="Group {0}/{1}: {2}/{3}"
             data-paste-error="Paste rejected"
             data-format-error="Format rejected"
             data-label-id="pin-label"
             data-hint-id="pin-hint"
             data-error-id="pin-error">
          <input id="password" name="password" type="password" tabindex="3" value="${password}">
          <button type="button" data-structured-credential-toggle
                  data-label-show="Show" data-label-hide="Hide"
                  data-icon-show="show" data-icon-hide="hide"><i></i></button>
        </div>
        <div id="pin-hint">Hint</div>
        <span id="pin-error" hidden>Wrong credential</span>
        <button id="submit" type="submit" name="login" value="yes">Login</button>
      </form>`,
    { runScripts: "outside-only", url: "https://example.test" },
  );
  dom.window.eval(script);
  return {
    container: dom.window.document.querySelector("[data-structured-credential]"),
    display: dom.window.document.getElementById("structured-password"),
    dom,
    error: dom.window.document.getElementById("pin-error"),
    form: dom.window.document.getElementById("login"),
    real: dom.window.document.getElementById("password"),
    status: dom.window.document.getElementById("structured-credential-status"),
    submit: dom.window.document.getElementById("submit"),
    toggle: dom.window.document.querySelector("[data-structured-credential-toggle]"),
  };
};

const typeDigit = (dom, input, digit) => {
  input.dispatchEvent(
    new dom.window.InputEvent("beforeinput", {
      bubbles: true,
      cancelable: true,
      data: digit,
      inputType: "insertText",
    }),
  );
};

const paste = (dom, input, value) => {
  const event = new dom.window.Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "clipboardData", {
    value: { getData: () => value },
  });
  input.dispatchEvent(event);
};

const submitEvent = (dom, submitter) => {
  const event = new dom.window.Event("submit", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "submitter", { value: submitter });
  return event;
};

test("enhances one visible input while retaining the ordinary named password field", () => {
  const { container, display, real } = setup();

  assert.equal(container.dataset.structuredCredentialEnhanced, "true");
  assert.equal(display.inputMode, "numeric");
  assert.equal(display.autocomplete, "current-password");
  assert.equal(display.tabIndex, 3);
  assert.equal(real.type, "hidden");
  assert.equal(real.name, "password");
  assert.equal(display.name, "");
});

test("malformed patterns preserve the native password input and visibility toggle", () => {
  const { display, real, toggle } = setup({ pattern: "dddd-text" });

  assert.equal(display, null);
  assert.equal(real.type, "password");
  toggle.click();
  assert.equal(real.type, "text");
});

test("typing fills groups, arrow navigation replaces a selected group, and extra digits are ignored", () => {
  const { display, dom, real } = setup();
  display.focus();
  for (const digit of "12345678") {
    typeDigit(dom, display, digit);
  }
  assert.equal(real.value, "12345678");

  typeDigit(dom, display, "9");
  assert.equal(real.value, "12345678");

  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "ArrowLeft",
    }),
  );
  typeDigit(dom, display, "9");
  assert.equal(display.value, "9ddd-****");
  assert.equal(real.value, "95678");
});

test("password-manager replacement input is accepted and synchronized", () => {
  const { display, dom, real } = setup();
  display.value = "1234-5678";
  display.dispatchEvent(
    new dom.window.InputEvent("input", {
      bubbles: true,
      data: null,
      inputType: "insertReplacementText",
    }),
  );

  assert.equal(real.value, "12345678");
  assert.match(display.value, /^\*{4}-\*{3}8$/);
});

test("plain input events and late hidden-field autofill are synchronized", () => {
  const { display, dom, real } = setup();
  display.value = "1234-5678";
  display.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(real.value, "12345678");

  real.value = "87654321";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(real.value, "87654321");
  assert.match(display.value, /^\*{4}-\*{3}1$/);
});

test("invalid hidden-field autofill is rejected, restored, and shown", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "123";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  real.dispatchEvent(new dom.window.Event("change", { bubbles: true }));

  assert.equal(real.value, "12345678");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");

  const username = form.querySelector('input[name="username"]');
  username.value = "another-voter";
  username.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(error.hidden, false);
  assert.equal(username.hasAttribute("aria-invalid"), false);
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("invalid visible replacement shows format error and editing restores the generic error", () => {
  const { display, dom, error, form, submit } = setup();
  const insertion = new dom.window.InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    data: null,
    inputType: "insertText",
  });
  assert.equal(display.dispatchEvent(insertion), false);
  assert.equal(display.value, "dddd-dddd");
  assert.equal(error.hidden, true);

  display.value = "not a PIN";
  display.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(display.value, "dddd-dddd");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");

  typeDigit(dom, display, "1");
  assert.equal(error.hidden, true);
  assert.equal(error.textContent, "Wrong credential");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Wrong credential");
});

test("eventless password-manager values are reconciled on submit", () => {
  const { dom, form, real, submit } = setup();
  real.value = "12345678";

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), true);
  assert.equal(real.value, "12345678");
});

test("eventless malformed hidden autofill is restored and shown on submit", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "invalid";

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(real.value, "12345678");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("paired hidden clear events remain pending and cannot submit the stale PIN", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  real.dispatchEvent(new dom.window.Event("change", { bubbles: true }));

  assert.equal(real.value, "");
  assert.equal(error.hidden, true);
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(real.value, "");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");
  real.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("password-manager clear then same valid fill resolves the pending invalid state", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  real.value = "12345678";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));

  assert.equal(real.value, "12345678");
  assert.equal(error.hidden, true);
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), true);
});

test("clear then malformed paired events cannot submit the stale PIN", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  real.value = "invalid";
  real.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  real.dispatchEvent(new dom.window.Event("change", { bubbles: true }));

  assert.equal(real.value, "invalid");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(real.value, "invalid");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("eventless hidden-field clearing cannot submit a previously complete credential", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  real.value = "";

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(real.value, "");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("eventless malformed visible replacement is restored and shown on submit", () => {
  const { display, dom, error, form, real, submit } = setup();
  paste(dom, display, "1234-5678");
  display.value = "invalid";

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.notEqual(display.value, "invalid");
  assert.equal(real.value, "12345678");
  assert.equal(error.hidden, false);
  assert.equal(error.textContent, "Format rejected");
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
});

test("paste distributes a complete credential and announces rejected values", () => {
  const { display, dom, real, status } = setup();
  paste(dom, display, "1234-5678");
  assert.equal(real.value, "12345678");

  paste(dom, display, "not a PIN");
  assert.equal(real.value, "12345678");
  assert.equal(status.textContent, "Paste rejected");

  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "ArrowLeft",
    }),
  );
  assert.equal(status.textContent, "Group 1/2: 4/4");
});

test("partial paste replaces its groups without clearing later groups", () => {
  const { display, dom, real } = setup();
  paste(dom, display, "1234-5678");
  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "ArrowLeft",
    }),
  );
  paste(dom, display, "99");

  assert.equal(real.value, "995678");
  assert.match(display.value, /^\*9dd-\*{4}$/);
});

test("Backspace and Delete clear selected groups", () => {
  const { display, dom, real } = setup();
  paste(dom, display, "1234-5678");
  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "Backspace",
    }),
  );
  assert.equal(real.value, "1234");

  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "ArrowLeft",
    }),
  );
  display.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", {
      bubbles: true,
      cancelable: true,
      key: "Delete",
    }),
  );
  assert.equal(real.value, "");
});

test("the last digit is remasked and page hiding closes visibility", async () => {
  const { display, dom, toggle } = setup();
  typeDigit(dom, display, "1");
  assert.equal(display.value, "1ddd-dddd");
  await new Promise((resolve) => dom.window.setTimeout(resolve, 1050));
  assert.equal(display.value, "*ddd-dddd");

  toggle.click();
  assert.equal(display.value, "1ddd-dddd");
  dom.window.dispatchEvent(new dom.window.Event("pagehide"));
  assert.equal(display.value, "*ddd-dddd");
  assert.equal(toggle.getAttribute("aria-pressed"), "false");
});

test("unrelated field edits retain errors and a one-digit pattern remains valid", () => {
  const { display, dom, error, form, real, submit } = setup({ pattern: "d" });
  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  assert.equal(error.hidden, false);

  const unrelated = dom.window.document.createElement("input");
  unrelated.name = "firstName";
  form.append(unrelated);
  unrelated.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  assert.equal(error.hidden, false);

  typeDigit(dom, display, "7");
  assert.equal(real.value, "7");
});

test("incomplete submit is blocked and complete submit resets after bfcache restoration", async () => {
  const { display, dom, error, form, real, submit } = setup();
  for (const digit of "1234") {
    typeDigit(dom, display, digit);
  }
  const incomplete = submitEvent(dom, submit);
  assert.equal(form.dispatchEvent(incomplete), false);
  assert.equal(error.hidden, false);

  paste(dom, display, "1234-5678");
  const complete = submitEvent(dom, submit);
  assert.equal(form.dispatchEvent(complete), true);
  assert.equal(real.value, "12345678");
  assert.equal(submit.disabled, false);

  const duplicate = submitEvent(dom, submit);
  assert.equal(form.dispatchEvent(duplicate), false);
  assert.equal(submit.disabled, false);

  await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
  assert.equal(submit.disabled, true);
  dom.window.dispatchEvent(new dom.window.Event("pageshow"));
  assert.equal(submit.disabled, false);

  const restored = submitEvent(dom, submit);
  assert.equal(form.dispatchEvent(restored), true);
});

test("later submit cancellation releases the re-entry guard and leaves feedback enabled", async () => {
  const { display, dom, form, submit } = setup();
  paste(dom, display, "1234-5678");
  let cancellations = 0;
  form.addEventListener("submit", (event) => {
    cancellations += 1;
    event.preventDefault();
  });

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
  assert.equal(submit.disabled, false);

  assert.equal(form.dispatchEvent(submitEvent(dom, submit)), false);
  await new Promise((resolve) => dom.window.setTimeout(resolve, 0));
  assert.equal(submit.disabled, false);
  assert.equal(cancellations, 2);
});
