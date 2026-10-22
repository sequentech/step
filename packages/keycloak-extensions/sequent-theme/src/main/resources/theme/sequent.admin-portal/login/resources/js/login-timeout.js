// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Keycloak rejects a login form submitted after the realm's login timeout
// and makes the voter start again. Ask Keycloak how long this login has left,
// and when the page is used after that, start the login over before the voter
// types, keeping the username they had already typed.

// Covers the time the form takes to reach Keycloak.
const SUBMIT_MARGIN_MS = 5 * 1000;
const RESTART_KEY = "sequent.loginTimeoutRestart";
// A restart older than this belongs to some other page and is ignored.
const RESTART_MAX_AGE_MS = 60 * 1000;
// When Keycloak can't be reached, ask again after this long.
const RETRY_MS = 30 * 1000;
const AUTH_ENDPOINT_PATH = /\/protocol\/openid-connect\/auth$/;
const LOGIN_ACTIONS_PATH = "/login-actions/";
const STATUS_PATH = "/login-timeout";
// Only fields that identify the voter come back after a restart: never
// secrets, codes or personal data.
const KEPT_FIELD_NAMES = ["username", "email"];
const LOGIN_PARAMS = ["client_id", "tab_id"];

const script = document.currentScript;
const loginAction = new URL(script.dataset.loginAction || location.href, location.href);
let deadline = null;
let deadlineTimer = null;
let checking = false;
let restarting = false;
// Cleared when a page that a restart just opened is already expired: restarting
// again would loop, so that is left to Keycloak.
let mayRestart = true;
let firstCheck = true;

// Both URLs name this page's login, so Keycloak doesn't fall back to
// whichever login the browser started last.
function withLoginParams(url) {
  LOGIN_PARAMS.forEach((param) => {
    const value = loginAction.searchParams.get(param);
    if (value) {
      url.searchParams.set(param, value);
    }
  });
  return url;
}

const restartUrl = withLoginParams(new URL(script.dataset.restartUrl, location.href));
const statusUrl = new URL(
  restartUrl.pathname.slice(0, restartUrl.pathname.indexOf(LOGIN_ACTIONS_PATH)) + STATUS_PATH,
  restartUrl,
);
const tabId = loginAction.searchParams.get("tab_id");
if (tabId) {
  statusUrl.searchParams.set("tab_id", tabId);
}

function keptFields() {
  return KEPT_FIELD_NAMES.map((name) => document.querySelector(`form input[name="${name}"]`))
    .filter((field) => field && !field.disabled && field.type !== "password");
}

function takeSavedRestart() {
  try {
    const saved = JSON.parse(sessionStorage.getItem(RESTART_KEY));
    sessionStorage.removeItem(RESTART_KEY);
    return saved && Date.now() - saved.savedAt < RESTART_MAX_AGE_MS ? saved : null;
  } catch (e) {
    return null;
  }
}

// Read once, before anything can restart this page again.
const savedRestart = takeSavedRestart();

function restart() {
  if (restarting || !mayRestart) {
    return;
  }
  restarting = true;
  const restartFlow = !AUTH_ENDPOINT_PATH.test(location.pathname);
  const values = {};
  keptFields().forEach((field) => {
    if (field.value) {
      values[field.name] = field.value;
    }
  });
  try {
    sessionStorage.setItem(
      RESTART_KEY,
      JSON.stringify({savedAt: Date.now(), hideNotice: restartFlow, values}),
    );
  } catch (e) {
    // Storage blocked: the voter types again, as without this script.
  }
  if (restartFlow) {
    // A form result or a later step: reloading would resubmit the form, so
    // use Keycloak's restart endpoint, whose "timed out" notice the next page
    // hides. It keeps any session the voter is already signed in with.
    location.replace(restartUrl);
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
  if (!savedRestart) {
    return;
  }
  if (savedRestart.hideNotice) {
    document.querySelectorAll("[data-login-timeout-notice]").forEach((notice) => {
      notice.style.display = "none";
    });
  }
  const values = savedRestart.values || {};
  keptFields().forEach((field) => {
    if (!field.value && typeof values[field.name] === "string") {
      setValue(field, values[field.name]);
    }
  });
}

function expired() {
  return deadline !== null && Date.now() >= deadline;
}

// Asks Keycloak again each time, since another tab of the same login also
// extends it. When Keycloak has no login for this page, it is left alone.
// Until Keycloak answers, the last deadline it gave still applies.
function check() {
  if (restarting || checking || document.visibilityState !== "visible") {
    return;
  }
  checking = true;
  fetch(statusUrl, {credentials: "same-origin", cache: "no-store"})
    .then((response) => {
      if (response.status === 204) {
        return null;
      }
      if (!response.ok) {
        throw new Error(`login-timeout answered ${response.status}`);
      }
      return response.json();
    })
    .then((status) => {
      clearTimeout(deadlineTimer);
      if (!status || !Number.isFinite(status.secondsLeft)) {
        deadline = null;
        return;
      }
      deadline = Date.now() + status.secondsLeft * 1000 - SUBMIT_MARGIN_MS;
      if (!expired()) {
        deadlineTimer = setTimeout(check, deadline - Date.now());
      } else if (firstCheck && savedRestart) {
        mayRestart = false;
      } else {
        restart();
      }
    })
    .catch(() => {
      // A failed request must not switch the check off, nor push it past the
      // deadline Keycloak already gave.
      clearTimeout(deadlineTimer);
      const untilDeadline = deadline === null ? 0 : deadline - Date.now();
      deadlineTimer = setTimeout(
        check,
        untilDeadline > 0 ? Math.min(RETRY_MS, untilDeadline) : RETRY_MS,
      );
    })
    .finally(() => {
      checking = false;
      firstCheck = false;
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
    if (expired() && mayRestart) {
      event.preventDefault();
      event.stopImmediatePropagation();
      restart();
    }
  },
  true,
);
