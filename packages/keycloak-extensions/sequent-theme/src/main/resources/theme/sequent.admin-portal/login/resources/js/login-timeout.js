// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Keycloak rejects a login form submitted after the realm's login timeout
// and makes the voter start again. Ask Keycloak how long this login has left,
// and when the page is used after that, start the login over before the voter
// types, keeping what they had already typed.

// Covers the time the form takes to reach Keycloak.
const SUBMIT_MARGIN_MS = 5 * 1000;
const RESTART_KEY = "sequent.loginTimeoutRestart";
// Values older than this belong to some other restart and are dropped.
const RESTART_MAX_AGE_MS = 60 * 1000;
const AUTH_ENDPOINT_PATH = /\/protocol\/openid-connect\/auth$/;
const FIELDS = "form input, form select, form textarea";
const KEPT_INPUT_TYPES = ["text", "email", "tel", "number", "date", "search", "url"];
const SECRET_AUTOCOMPLETE = /password|one-time-code|cc-/;

const script = document.currentScript;
let deadline = null;
let deadlineTimer = null;
let checking = null;
let restarting = false;

function statusUrl() {
  const url = new URL(script.dataset.statusUrl, location.href);
  if (script.dataset.tabId) {
    url.searchParams.set("tab_id", script.dataset.tabId);
  }
  return url;
}

// Only plain identifying fields are kept: never passwords, codes or hidden
// values.
function isKeptField(field) {
  if (!field.name || field.disabled || SECRET_AUTOCOMPLETE.test(field.autocomplete || "")) {
    return false;
  }
  if (field instanceof HTMLInputElement) {
    return KEPT_INPUT_TYPES.includes(field.type);
  }
  return field instanceof HTMLSelectElement || field instanceof HTMLTextAreaElement;
}

function typedValues() {
  const values = {};
  document.querySelectorAll(FIELDS).forEach((field) => {
    if (field.value && isKeptField(field)) {
      values[field.name] = field.value;
    }
  });
  return values;
}

function restart() {
  if (restarting) {
    return;
  }
  restarting = true;
  const restartFlow = !AUTH_ENDPOINT_PATH.test(location.pathname);
  try {
    sessionStorage.setItem(
      RESTART_KEY,
      JSON.stringify({savedAt: Date.now(), hideNotice: restartFlow, values: typedValues()}),
    );
  } catch (e) {
    // Storage blocked: the voter types again, as without this script.
  }
  if (restartFlow) {
    // A form result or a later step: reloading would resubmit the form, so
    // use Keycloak's restart endpoint, whose "timed out" notice the next page
    // hides.
    location.replace(script.dataset.restartUrl);
  } else {
    // First page of the flow: reloading the authorization request opens a
    // fresh login without any notice.
    location.reload();
  }
}

function setValue(field, value) {
  // Go through the native setter so frameworks that track the value notice.
  Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field), "value").set.call(field, value);
  field.dispatchEvent(new Event("input", {bubbles: true}));
  field.dispatchEvent(new Event("change", {bubbles: true}));
}

function resumeAfterRestart() {
  let saved = null;
  try {
    saved = JSON.parse(sessionStorage.getItem(RESTART_KEY));
    sessionStorage.removeItem(RESTART_KEY);
  } catch (e) {
    return;
  }
  if (!saved || !(Date.now() - saved.savedAt < RESTART_MAX_AGE_MS)) {
    return;
  }
  if (saved.hideNotice) {
    document.querySelectorAll("[data-login-timeout-notice]").forEach((notice) => {
      notice.style.display = "none";
    });
  }
  const values = saved.values || {};
  document.querySelectorAll(FIELDS).forEach((field) => {
    if (Object.hasOwn(values, field.name) && !field.value && isKeptField(field)) {
      setValue(field, String(values[field.name]));
    }
  });
}

function expired() {
  return deadline !== null && Date.now() >= deadline;
}

// Asks Keycloak again each time, since another tab of the same login also
// extends it. If Keycloak can't be asked, the page is left alone.
function check() {
  if (restarting || document.visibilityState !== "visible") {
    return;
  }
  if (checking) {
    return;
  }
  checking = fetch(statusUrl(), {credentials: "same-origin", cache: "no-store"})
    .then((response) => (response.ok ? response.json() : null))
    .then((status) => {
      if (!status || !Number.isFinite(status.secondsLeft)) {
        return;
      }
      deadline = Date.now() + status.secondsLeft * 1000 - SUBMIT_MARGIN_MS;
      clearTimeout(deadlineTimer);
      if (expired()) {
        restart();
      } else {
        deadlineTimer = setTimeout(check, deadline - Date.now());
      }
    })
    .catch(() => {})
    .finally(() => {
      checking = null;
    });
}

resumeAfterRestart();
check();

document.addEventListener("visibilitychange", check);
window.addEventListener("focus", check);
window.addEventListener("pageshow", (event) => {
  if (event.persisted) {
    check();
  }
});
// Last resort: a form Keycloak would reject must never be sent.
document.addEventListener(
  "submit",
  (event) => {
    if (expired()) {
      event.preventDefault();
      event.stopImmediatePropagation();
      restart();
    }
  },
  true,
);
