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
