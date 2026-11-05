// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, beforeEach, expect} from "vitest"
import {server} from "vitest/browser"

declare module "vitest/browser" {
    interface BrowserCommands {
        startNetworkGuard: () => Promise<void>
        finishNetworkGuard: () => Promise<string[]>
    }
}

beforeEach(async () => {
    await server.commands.startNetworkGuard()
})
afterEach(async () => {
    expect(await server.commands.finishNetworkGuard(), "Unexpected network requests").toEqual([])
})
