// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {mergeConfig} from "vitest/config"
import {playwright} from "@vitest/browser-playwright"
import {createStorybookTests, reactBrowserDependencies} from "../test-support/storybook/vitest"

// Unit tests run in Chromium too: the capture code uses canvas, File and DataTransfer.
export default mergeConfig(createStorybookTests(new URL("./.storybook", import.meta.url)), {
    test: {
        projects: [
            {
                plugins: [reactBrowserDependencies],
                test: {
                    name: "unit",
                    include: ["src/**/*.test.ts"],
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: playwright({
                            launchOptions: {
                                executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
                            },
                            contextOptions: {locale: "en-US", timezoneId: "UTC"},
                        }),
                        instances: [{browser: "chromium"}],
                    },
                },
            },
        ],
    },
})
