// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What every test in this package gets before it runs.
 *
 * `@testing-library/jest-dom` for the matchers, and two jsdom gaps that MUI
 * reaches for. Both are stubbed rather than left to fail because the failure is
 * a thrown `TypeError` from inside a component, which reads as a broken
 * component rather than a missing browser API.
 */

// This package's tsconfig sets `"types": []`, which switches off automatic
// `@types` inclusion so a component library does not compile against ambient Node
// globals it has no business using. That is right for `src/components`, and it is
// why this one test-support file opts itself in explicitly rather than the setting
// being relaxed for everything.
/// <reference types="node" />

import "@testing-library/jest-dom"
import {TextDecoder, TextEncoder} from "node:util"

// jsdom's environment does not expose these, and `react-router` reads them at
// import time — reached from this package's theme, which sets `LinkBehavior` as the
// default component for `MuiLink`. So a test that only wants a ballot still pulls
// the router in. `EA-F1-005` cuts that edge for the shared ballot entry; until then
// the polyfill is what makes the theme importable at all.
//
// From `node:util`, not off `global`. Reading `global.TextEncoder` was the first
// attempt and fails in a way worth recording: under `jest-environment-jsdom`,
// `global` *is* the jsdom window, where `TextEncoder` does not exist — so the
// assignment set `undefined` and the failure surfaced two files away as
// "TextEncoder is not a constructor" from inside `react-router`. The import costs
// `@types/node` as a devDependency, which a package that runs jest can own.
if (typeof globalThis.TextEncoder === "undefined") {
    Object.assign(globalThis, {TextEncoder, TextDecoder})
}

// MUI's `useMediaQuery` calls this on mount. jsdom has no implementation, and
// without it every responsive component throws on first render. Reports "no
// match", which is the correct answer for jsdom's 1024px-wide window at the
// breakpoints this app uses.
if (typeof window.matchMedia !== "function") {
    Object.defineProperty(window, "matchMedia", {
        writable: true,
        value: (query: string) => ({
            matches: false,
            media: query,
            onchange: null,
            addListener: () => undefined,
            removeListener: () => undefined,
            addEventListener: () => undefined,
            removeEventListener: () => undefined,
            dispatchEvent: () => false,
        }),
    })
}

// Not in jsdom, and MUI's transitions and `Popper` use it.
if (typeof window.ResizeObserver !== "function") {
    window.ResizeObserver = class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
    } as unknown as typeof ResizeObserver
}

// Node has had `structuredClone` as a global since 17, but jsdom does not
// install it on its own `window`, so a test that clones a fixture works under
// `testEnvironment: "node"` and throws `structuredClone is not defined` under
// jsdom. Moving the voting portal to jsdom — so its ballot components could be
// mounted at all — is what surfaced it, in a slice test that arrived from main
// at the same time.
//
// Node's own implementation, borrowed rather than reimplemented: a hand-rolled
// deep clone would differ from the real one on exactly the values a fixture is
// least likely to contain and most awkward to debug.
if (typeof globalThis.structuredClone !== "function") {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const {serialize, deserialize} = require("node:v8")
    globalThis.structuredClone = ((value: unknown) =>
        deserialize(serialize(value))) as typeof structuredClone
}

// `react-dom/server` reaches for `MessageChannel` as it loads — React's
// scheduler uses it to yield between units of work — and this jsdom does not
// provide one. The failure is a *collection* error, so a suite that imports
// `renderToStaticMarkup` never runs at all and jest reports "0 failed" beside
// it: five of this package's six suites were dark that way, which is not a
// state to start restructuring the voting path from.
//
// Node's own, from `worker_threads`. Its ports are `EventTarget`s and support
// `onmessage`, which is the whole of what the scheduler asks for.
if (typeof globalThis.MessageChannel !== "function") {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    globalThis.MessageChannel = require("node:worker_threads")
        .MessageChannel as typeof MessageChannel
}
