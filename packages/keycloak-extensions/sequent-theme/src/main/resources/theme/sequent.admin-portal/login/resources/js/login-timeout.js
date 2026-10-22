// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Keycloak rejects a login form submitted after the realm's login timeout
// and makes the voter start again. When a page that has been open longer
// than that becomes active again, start the login over before they type.

const SAFETY_MARGIN_MS = 60 * 1000;
const CHECK_INTERVAL_MS = 30 * 1000;
const RESTART_FLAG = "sequent.loginTimeoutRestart";
// Keycloak checks these pages against the shorter "login action timeout".
const USER_ACTION_PATH = /\/login-actions\/(required-action|reset-credentials)$/;
const AUTH_ENDPOINT_PATH = /\/protocol\/openid-connect\/auth$/;

function seconds(value, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

const script = document.currentScript;
const renderedAt = Date.now();
const timeoutSeconds = USER_ACTION_PATH.test(location.pathname)
  ? seconds(script.dataset.actionTimeoutSeconds, 300)
  : seconds(script.dataset.timeoutSeconds, 1800);
// Short timeouts keep half of their length as the margin.
const limitMs = timeoutSeconds * 1000 - Math.min(SAFETY_MARGIN_MS, (timeoutSeconds * 1000) / 2);
let restarting = false;

function restartIfExpired() {
  if (restarting || Date.now() - renderedAt < limitMs) {
    return false;
  }
  restarting = true;
  if (AUTH_ENDPOINT_PATH.test(location.pathname)) {
    // First page of the flow: reloading the authorization request opens a
    // fresh login session without any notice.
    location.reload();
  } else {
    // A form result or a later step: reloading would resubmit the form, so
    // use Keycloak's restart endpoint, which shows a "timed out" notice that
    // the next page hides.
    try {
      sessionStorage.setItem(RESTART_FLAG, "1");
    } catch (e) {
      // Storage blocked: the notice stays visible, which is harmless.
    }
    location.replace(script.dataset.restartUrl);
  }
  return true;
}

function hideOwnTimeoutNotice() {
  let restarted = null;
  try {
    restarted = sessionStorage.getItem(RESTART_FLAG);
    sessionStorage.removeItem(RESTART_FLAG);
  } catch (e) {
    return;
  }
  if (restarted) {
    document.querySelectorAll("[data-login-timeout-notice]").forEach((notice) => {
      notice.style.display = "none";
    });
  }
}

hideOwnTimeoutNotice();

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    restartIfExpired();
  }
});
window.addEventListener("focus", restartIfExpired);
window.addEventListener("pageshow", (event) => {
  if (event.persisted) {
    restartIfExpired();
  }
});
// Timers are throttled in background tabs and stop while the computer sleeps,
// so this only covers a page left in view.
setInterval(() => {
  if (document.visibilityState === "visible") {
    restartIfExpired();
  }
}, Math.min(CHECK_INTERVAL_MS, limitMs / 2));
// Last resort: an expired page must never submit.
document.addEventListener(
  "submit",
  (event) => {
    if (restartIfExpired()) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  },
  true,
);
