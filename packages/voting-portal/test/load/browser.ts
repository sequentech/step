// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {LaunchOptions} from "@playwright/test"

/** Docker-only HTTP origins need secure-context APIs for WASM and OIDC. */
export function browserOptions(): LaunchOptions {
    return {
        headless: true,
        // Use full Chromium's modern headless mode, including secure-context flags.
        channel: "chromium",
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
        args: process.env.E2E_LOCAL_STACK === "1"
            ? ["--unsafely-treat-insecure-origin-as-secure=http://portals:3000,http://portals:3001,http://portals:3002,http://portals:3004"]
            : [],
    }
}
