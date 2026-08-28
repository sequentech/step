// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import "cross-fetch/polyfill"
import {TextDecoder, TextEncoder} from "node:util"
import {deserialize, serialize} from "node:v8"

// jsdom leaves these out of its global object even though both browsers and node
// provide them, and the code under test relies on them: react-router reads
// TextEncoder when it loads, and the ballot selection slices use structuredClone.
// Referenced from jest.config.cjs as setupFiles, so they exist before any module
// under test is imported.
const polyfills: Record<string, unknown> = {
    TextEncoder,
    TextDecoder,
    structuredClone: <T>(value: T): T => deserialize(serialize(value)) as T,
}

Object.entries(polyfills).forEach(([name, implementation]) => {
    if (name in globalThis) {
        return
    }
    Object.defineProperty(globalThis, name, {
        value: implementation,
        writable: true,
        configurable: true,
    })
})
