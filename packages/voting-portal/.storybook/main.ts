// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {mergeConfig} from "vite"
import type {StorybookConfig} from "@storybook/react-vite"
import config from "../../ui-essentials/.storybook/main.ts"

export default {
    ...config,
    staticDirs: ["../public"],
    viteFinal: async (viteConfig, options) =>
        mergeConfig(await config.viteFinal!(viteConfig, options), {
            // Screen stories import the auth context; discover its SDK before tests start.
            optimizeDeps: {include: ["keycloak-js"]},
        }),
} satisfies StorybookConfig
