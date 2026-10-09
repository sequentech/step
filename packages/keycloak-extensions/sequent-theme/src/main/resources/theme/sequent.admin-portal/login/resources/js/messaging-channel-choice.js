// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// One channel per voter, among the selected Post's channels.
// - Shows only the channels the selected Post offers (data-channels-by-post, keyed by the Post
//   field's value) and refreshes them when the Post changes. A channel the new Post doesn't offer
//   is cleared. The server checks the submitted Post and channel again.
// - Says "The ways {post} can reach you." when the Post offers fewer channels than the event.
// - Shows the number and consent of the selected channel only, and unticks the boxes of the other
//   channels, so a consent never applies to a channel it did not name.
const choice = document.querySelector("[data-messaging-channel-choice]")

function channelsByPost() {
    try {
        return JSON.parse(choice.dataset.channelsByPost || "{}")
    } catch (error) {
        console.warn("The channels by Post could not be read; showing the page's channels.")
        return {}
    }
}

function postField() {
    const name = choice.dataset.postAttribute
    return name ? document.querySelector(`[name="${CSS.escape(name)}"]`) : null
}

function postLabel(field) {
    if (field && field.tagName === "SELECT" && field.selectedIndex >= 0) {
        return field.options[field.selectedIndex].text
    }
    return field ? field.value : ""
}

function showPostChannels() {
    const field = postField()
    const byPost = channelsByPost()
    const post = field ? field.value : ""
    const options = choice.querySelectorAll("[data-messaging-option]")
    // A Post the map doesn't know keeps the channels the page was rendered with.
    const offered = Object.prototype.hasOwnProperty.call(byPost, post) ? byPost[post] : null
    options.forEach((option) => {
        if (offered === null) {
            return
        }
        const shown = offered.includes(option.dataset.messagingOption)
        option.hidden = !shown
        const radio = option.querySelector("input[type='radio']")
        if (!shown && radio && radio.checked) {
            radio.checked = false
        }
    })
    const help = document.getElementById("messaging-post-help")
    if (help) {
        const fewer = offered !== null && post && offered.length < options.length
        help.hidden = !fewer
        help.textContent = fewer
            ? (help.dataset.postHelpTemplate || "").replace("__post__", postLabel(field))
            : ""
    }
    showSelected()
}

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
    const field = postField()
    if (field) {
        field.addEventListener("change", showPostChannels)
    }
    showPostChannels()
}
