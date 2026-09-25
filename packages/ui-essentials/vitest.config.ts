// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {fileURLToPath} from "node:url"
import {defineConfig} from "vitest/config"
import {storybookTest} from "@storybook/addon-vitest/vitest-plugin"
import {playwright} from "@vitest/browser-playwright"

// Every story file runs as a test in headless Chromium, with a fixed locale and
// time zone so that rendered dates do not depend on the machine.
export default defineConfig({
    test: {
        projects: [
            {
                extends: true,
                plugins: [
                    storybookTest({configDir: fileURLToPath(new URL(".storybook", import.meta.url))}),
                ],
                test: {
                    name: "storybook",
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: playwright({
                            contextOptions: {locale: "en-US", timezoneId: "UTC"},
                        }),
                        instances: [{browser: "chromium"}],
                    },
                },
            },
        ],
    },
})
