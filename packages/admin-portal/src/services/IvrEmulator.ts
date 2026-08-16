// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {loadIvrEmulator as load} from "@sequentech/ui-essentials"
import type {IvrEmulatorApi} from "@sequentech/ui-essentials"

export {IvrEmulatorError} from "@sequentech/ui-essentials"
export type {IvrEmulatorApi} from "@sequentech/ui-essentials"

export type {
    IvrEmulatorDriver,
    Action,
    PromptInfo,
    EmulatorConfig,
} from "./generated/ivr_emulator_wasm"

/**
 * Bring in the generated shim, without the bundler following it.
 *
 * The fetch-and-init itself is in `@sequentech/ui-essentials`, shared with the
 * Election Architect, which offers a call preview over the plan on screen. This one
 * line cannot go with it: the comments are instructions to *whichever bundler
 * compiles this file*, and a comment in a library is compiled away long before an
 * application's bundler sees the import. So the library takes the import as a
 * parameter and each host supplies its own line, with the comments its own bundler
 * reads.
 */
const importShim = (href: string): Promise<unknown> =>
    import(
        /* @vite-ignore */
        /* webpackIgnore: true */
        href
    )

export const loadIvrEmulator = (baseUrl: string): Promise<IvrEmulatorApi> =>
    load(baseUrl, importShim)
