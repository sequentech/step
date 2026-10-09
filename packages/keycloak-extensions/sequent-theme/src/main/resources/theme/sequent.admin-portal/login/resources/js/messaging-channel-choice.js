// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Shows the number, consent and notice choice of the selected channel only. Unticks the boxes of
// the other channels, so a consent or notice choice never applies to a channel it did not name.
const choice = document.querySelector("[data-messaging-channel-choice]")

function showSelected() {
    const selected = choice.querySelector("input[type='radio']:checked")?.value
    document.querySelectorAll("[data-messaging-for]").forEach((details) => {
        const shown = details.dataset.messagingFor === selected
        details.hidden = !shown
        if (!shown) {
            details.querySelectorAll("input[type='checkbox']").forEach((box) => {
                box.checked = false
            })
        }
    })
}

if (choice) {
    choice.addEventListener("change", showSelected)
    showSelected()
}
