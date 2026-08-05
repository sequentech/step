// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Jest can't load the real sequent-core WASM bindings package: unlike the
// rest of node_modules it ships as plain ESM ("export function ..."), which
// trips the default CJS-oriented transform as soon as anything requires it
// (transitively, via @sequentech/ui-core). Unit tests that only exercise
// pure TS logic (e.g. Redux reducers) never call into the real WASM
// functions, so a permissive stub is enough to satisfy ui-core's
// module-load-time import of this package.
module.exports = new Proxy(
    {},
    {
        get: () => () => undefined,
    }
)
