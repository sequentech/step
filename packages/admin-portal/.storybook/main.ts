// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import config from "../../ui-essentials/.storybook/main.ts"
import postcssPresetEnv from "postcss-preset-env"
import {mergeConfig} from "vite"

const adminConfig = {
    ...config,
    staticDirs: ["../public"],
    viteFinal: async (viteConfig, options) =>
        mergeConfig(await config.viteFinal!(viteConfig, options), {
            css: {postcss: {plugins: [postcssPresetEnv()]}},
            optimizeDeps: {
                include: [
                    "ra-i18n-polyglot",
                    "ra-language-english",
                    "keycloak-js",
                    "@mui/icons-material/Download",
                    "@mui/icons-material/Upload",
                    "@mui/icons-material/ExpandMore",
                    "@mui/icons-material/Close",
                    "@mui/icons-material/ContentCopy",
                    "@mui/icons-material/Visibility",
                ],
            },
        }),
} satisfies StorybookConfig

export default adminConfig
