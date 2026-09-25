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
        }),
} satisfies StorybookConfig

export default adminConfig
