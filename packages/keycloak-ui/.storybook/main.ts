// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {fileURLToPath} from "node:url"
import {mergeConfig} from "vite"

const config: StorybookConfig = {
    stories: ["../src/**/*.stories.tsx"],
    staticDirs: ["../public"],
    addons: ["@storybook/addon-a11y", "@storybook/addon-docs", "@storybook/addon-vitest"],
    framework: "@storybook/react-vite",
    core: {disableTelemetry: true},
    viteFinal: (config) =>
        mergeConfig(config, {
            resolve: {
                alias: {
                    "@sequentech/ui-essentials/theme": fileURLToPath(
                        new URL("../../ui-essentials/src/services/theme.ts", import.meta.url)
                    ),
                },
                dedupe: ["react", "react-dom", "@emotion/react", "@mui/material"],
            },
        }),
}

export default config
