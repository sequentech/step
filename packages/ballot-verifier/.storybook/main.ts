// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {StorybookConfig} from "@storybook/react-vite"
import {fileURLToPath} from "node:url"
import config from "../../ui-essentials/.storybook/main.ts"

export default {
    ...config,
    staticDirs: ["../public"],
    framework: {
        name: "@storybook/react-vite",
        options: {
            builder: {
                viteConfigPath: fileURLToPath(new URL("./vite.config.ts", import.meta.url)),
            },
        },
    },
} satisfies StorybookConfig
