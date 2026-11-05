// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {TextDecoder, TextEncoder} from "node:util"

// React Router reads these browser APIs while it loads, and jsdom leaves them
// out, so install Node's implementations before any module under test runs.
Object.assign(globalThis, {TextDecoder, TextEncoder})

// jsdom has no fetch, and Apollo's HTTP link refuses to load without one. No
// test may reach the network: tests that expect requests stub this themselves.
globalThis.fetch = async (input) => {
    throw new Error(`Unexpected request in a unit test: ${String(input)}`)
}

// Browsers read an imported ballot with Blob.text(), which jsdom's Blob lacks.
if (!Blob.prototype.text) {
    Blob.prototype.text = function text(this: Blob) {
        return new Promise<string>((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => resolve(String(reader.result))
            reader.onerror = () => reject(reader.error)
            reader.readAsText(this)
        })
    }
}
