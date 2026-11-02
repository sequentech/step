// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// jsdom does not expose the WHATWG encoding classes, which @vitrine/verify
// uses to read and check trust policies.
const {TextEncoder, TextDecoder} = require("util")

if (typeof globalThis.TextEncoder === "undefined") {
    Object.assign(globalThis, {TextEncoder, TextDecoder})
}

// jsdom's File and Blob predate their browser text() methods.
if (typeof FileReader !== "undefined" && typeof Blob.prototype.text !== "function") {
    Blob.prototype.text = function () {
        return new Promise((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => resolve(String(reader.result))
            reader.onerror = () => reject(reader.error)
            reader.readAsText(this)
        })
    }
}
