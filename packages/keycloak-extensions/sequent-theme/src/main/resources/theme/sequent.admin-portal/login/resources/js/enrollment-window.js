// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Per-Post enrollment windows on the registration page (VOTE-LIFECYCLE).
// register.ftl renders one hidden notice per Post ([data-enrollment-embassy],
// with its state: before, open or closed). This shows the notice of the Post
// chosen in the embassy select and holds Continue back while that Post is not
// open. Continue is marked aria-disabled and its submit is blocked rather than
// using `disabled`, which the termsOfService checkbox handler also toggles on
// the same button. The enrollment-window-check form action refuses the
// registration on the server in any case.
(function () {
  "use strict";

  var BLOCKED_CLASS = "enrollment-window-blocked";

  function setup(doc) {
    var form = doc.getElementById("kc-register-form");
    var select = doc.getElementById("embassy");
    if (!form || !select) {
      return;
    }
    var notices = Array.prototype.slice.call(
      form.querySelectorAll("[data-enrollment-embassy]"),
    );
    var buttons = Array.prototype.slice.call(
      form.querySelectorAll("[type=submit]"),
    );
    var blocked = false;

    function update() {
      var chosen = null;
      notices.forEach(function (notice) {
        var match = notice.getAttribute("data-enrollment-embassy") === select.value;
        notice.hidden = !match || notice.textContent.trim() === "";
        if (match) {
          chosen = notice;
        }
      });
      blocked = chosen !== null && chosen.getAttribute("data-enrollment-state") !== "open";
      buttons.forEach(function (button) {
        button.classList.toggle(BLOCKED_CLASS, blocked);
        if (blocked) {
          button.setAttribute("aria-disabled", "true");
        } else {
          button.removeAttribute("aria-disabled");
        }
      });
    }

    form.addEventListener(
      "submit",
      function (event) {
        if (blocked) {
          event.preventDefault();
          event.stopImmediatePropagation();
        }
      },
      true,
    );
    // The country select picks the embassy programmatically, without a change
    // event on the embassy select: re-read after any change in the form.
    form.addEventListener("change", function () {
      setTimeout(update, 0);
    });
    update();
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = { setup: setup };
  }
  // register.ftl loads this after the form, so the form is already there.
  if (typeof document !== "undefined") {
    setup(document);
  }
})();
