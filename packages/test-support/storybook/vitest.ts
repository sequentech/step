// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {fileURLToPath} from "node:url"
import {defineConfig} from "vitest/config"
import {storybookTest} from "@storybook/addon-vitest/vitest-plugin"
import {startNetworkGuard, finishNetworkGuard} from "./network"
import {playwright} from "@vitest/browser-playwright"

// Every story file runs as a test in headless Chromium, with a fixed locale and
// time zone so that rendered dates do not depend on the machine.
export const createStorybookTests = (configDir: URL) =>
    defineConfig({
        test: {
            coverage: {
                provider: "istanbul",
                reportsDirectory: "test-results/coverage",
                reporter: ["text-summary", "json-summary", "html", "lcov"],
                include: ["src/**/*.{ts,tsx}"],
                exclude: [
                    "src/**/*.d.ts",
                    "src/**/*.test.{ts,tsx}",
                    "src/**/*.stories.{ts,tsx}",
                    "src/**/__stories__/**",
                ],
            },
            projects: [
                {
                    extends: true,
                    plugins: [
                        storybookTest({configDir: fileURLToPath(configDir)}),
                        {
                            name: "react-browser-dependencies",
                            enforce: "post",
                            configResolved(config) {
                                // Vitest discovers Nightwatch's unused Vue adapter.
                                // Only React stories run in this project.
                                for (const options of [
                                    config.optimizeDeps,
                                    ...Object.values(config.environments).map(
                                        (environment) => environment.optimizeDeps
                                    ),
                                ]) {
                                    options.include = options.include?.filter(
                                        (dependency) => dependency !== "@vue/test-utils"
                                    )
                                }
                            },
                        },
                    ],
                    test: {
                        name: "storybook",
                        maxWorkers: 1,
                        testTimeout: 15000,
                        clearMocks: true,
                        setupFiles: [
                            fileURLToPath(new URL("expectedFailures.ts", import.meta.url)),
                            fileURLToPath(new URL("networkSetup.ts", import.meta.url)),
                        ],
                        browser: {
                            commands: {startNetworkGuard, finishNetworkGuard},
                            enabled: true,
                            headless: true,
                            screenshotDirectory: "test-results/screenshots",
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
